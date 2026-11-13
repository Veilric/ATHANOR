use super::{SearchEntry, SearchSnapshot};
use crate::{cancel::Cancellation, feature::OpenTarget, storage::atomic_write_with};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

const MAX_ENTRIES: usize = 500_000;
const SKIP: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "__pycache__",
    "venv",
    "Library",
    "Caches",
];
pub type IndexError = io::Error;
pub fn build_file_snapshot(
    roots: &[PathBuf],
    cancel: &Cancellation,
) -> Result<SearchSnapshot, IndexError> {
    if cancel.is_cancelled() {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Indexing cancelled",
        ));
    }
    let mut stack: Vec<_> = roots
        .iter()
        .filter_map(|p| fs::canonicalize(p).ok())
        .collect();
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for root in roots {
        if !root.exists() {
            warning(
                &mut warnings,
                format!("Search folder is unavailable: {}", root.display()),
            );
        }
    }
    while let Some(dir) = stack.pop() {
        if cancel.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Indexing cancelled",
            ));
        }
        if !seen.insert(dir.clone()) {
            continue;
        }
        let children = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                warning(
                    &mut warnings,
                    format!("Cannot read {}: {error}", dir.display()),
                );
                continue;
            }
        };
        for child in children {
            if cancel.is_cancelled() {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Indexing cancelled",
                ));
            }
            let child = match child {
                Ok(entry) => entry,
                Err(error) => {
                    warning(&mut warnings, error.to_string());
                    continue;
                }
            };
            let name = child.file_name();
            let title = name.to_string_lossy();
            if title.starts_with('.') || SKIP.contains(&title.as_ref()) {
                continue;
            }
            let Ok(kind) = child.file_type() else {
                continue;
            };
            if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
                continue;
            }
            let path = child.path();
            if kind.is_dir() {
                stack.push(path.clone());
            }
            entries.push(SearchEntry {
                id: format!("{:?}", path.as_os_str()),
                title: title.into_owned(),
                location: path.clone(),
                target: OpenTarget::Path(path),
                is_folder: kind.is_dir(),
            });
            if entries.len() >= MAX_ENTRIES {
                warning(
                    &mut warnings,
                    "Index limit reached (500,000 entries). Narrow the search folders.".into(),
                );
                stack.clear();
                break;
            }
        }
    }
    let mut snapshot = SearchSnapshot::new(entries);
    snapshot.warnings = warnings;
    Ok(snapshot)
}
fn warning(warnings: &mut Vec<String>, message: String) {
    if warnings.len() < 32 {
        warnings.push(message);
    }
}
#[derive(Serialize, Deserialize)]
struct Header {
    version: u32,
    os: String,
}
pub fn write_cache(path: &Path, snapshot: &SearchSnapshot) -> io::Result<()> {
    atomic_write_with(path, |file| {
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(
            &mut writer,
            &Header {
                version: 1,
                os: std::env::consts::OS.into(),
            },
        )?;
        writer.write_all(b"\n")?;
        for entry in &snapshot.entries {
            serde_json::to_writer(&mut writer, entry.as_ref())?;
            writer.write_all(b"\n")?;
        }
        writer.flush()
    })
}
pub fn read_cache(path: &Path) -> io::Result<SearchSnapshot> {
    let file = File::open(path)?;
    if file.metadata()?.len() > 256 * 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Index cache is too large",
        ));
    }
    let mut lines = BufReader::new(file).lines();
    let header: Header = serde_json::from_str(
        &lines
            .next()
            .transpose()?
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing cache header"))?,
    )?;
    if header.version != 1 || header.os != std::env::consts::OS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Index cache has a different version or platform",
        ));
    }
    let mut entries = Vec::new();
    for line in lines.take(MAX_ENTRIES) {
        entries.push(serde_json::from_str(&line?)?);
    }
    Ok(SearchSnapshot::new(entries))
}
