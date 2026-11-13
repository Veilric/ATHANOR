use athanor_core::{feature::AngleMode, geometry::SavedPlacement, storage::atomic_write_with};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub opacity: f32,
    pub shortcut: String,
    pub search_roots: Vec<PathBuf>,
    pub preferred_display: Option<String>,
    pub positions: BTreeMap<String, SavedPlacement>,
    pub angle_mode: AngleMode,
    pub start_at_login: bool,
    /// X11 compositors can blend opaque GPU surfaces too; also dims text uniformly.
    pub x11_compositor_opacity: bool,
}
impl Default for Config {
    fn default() -> Self {
        let mut roots = Vec::new();
        if let Some(dirs) = directories::UserDirs::new() {
            for path in [dirs.document_dir(), dirs.desktop_dir(), dirs.download_dir()]
                .into_iter()
                .flatten()
            {
                if !roots.iter().any(|p| p == path) {
                    roots.push(path.to_path_buf());
                }
            }
        }
        Self {
            opacity: 0.65,
            shortcut: if cfg!(target_os = "macos") {
                "alt+space"
            } else {
                "ctrl+alt+space"
            }
            .into(),
            search_roots: roots,
            preferred_display: None,
            positions: BTreeMap::new(),
            angle_mode: AngleMode::Radians,
            start_at_login: false,
            x11_compositor_opacity: true,
        }
    }
}
impl Config {
    pub fn load(path: &Path) -> (Self, Option<String>) {
        if !path.exists() {
            return (Self::default(), None);
        }
        let result = (|| {
            if fs::metadata(path)?.len() > 64 * 1024 {
                return Err(io::Error::other("Settings file is too large"));
            }
            let config: Self = serde_json::from_slice(&fs::read(path)?)?;
            if !config.opacity.is_finite()
                || !(0.35..=1.0).contains(&config.opacity)
                || config.shortcut.is_empty()
                || config.shortcut.len() > 128
                || config.search_roots.len() > 32
                || config.positions.len() > 32
                || config
                    .positions
                    .values()
                    .any(|p| !p.offset.0.is_finite() || !p.offset.1.is_finite())
            {
                return Err(io::Error::other("Settings contain an invalid value"));
            }
            Ok(config)
        })();
        match result {
            Ok(config) => (config, None),
            Err(error) => (
                Self::default(),
                Some(format!(
                    "Using defaults; the existing settings file was preserved: {error}"
                )),
            ),
        }
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if path.exists() && Self::load(path).1.is_some() {
            let mut preserved = false;
            for number in 0..100 {
                let backup = path.with_extension(format!("invalid-{number}.json"));
                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(backup)
                {
                    Ok(mut file) => {
                        io::copy(&mut fs::File::open(path)?, &mut file)?;
                        file.sync_all()?;
                        preserved = true;
                        break;
                    }
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(error),
                }
            }
            if !preserved {
                return Err(io::Error::other(
                    "Cannot preserve the invalid settings file; existing data was left untouched",
                ));
            }
        }
        atomic_write_with(path, |file| {
            serde_json::to_writer_pretty(file, self).map_err(io::Error::other)
        })
    }
}
#[derive(Clone, Debug)]
pub struct AppPaths {
    pub data: PathBuf,
    pub config: PathBuf,
    pub index: PathBuf,
    pub rates: PathBuf,
}
impl AppPaths {
    pub fn discover() -> io::Result<Self> {
        let data = if let Some(path) = std::env::var_os("ATHANOR_DATA_DIR") {
            PathBuf::from(path)
        } else {
            directories::ProjectDirs::from("dev", "Veilric", "Athanor")
                .ok_or_else(|| io::Error::other("Cannot find the application data folder"))?
                .config_dir()
                .to_path_buf()
        };
        Ok(Self {
            config: data.join("config.json"),
            index: data.join("index.jsonl"),
            rates: data.join("rates.json"),
            data,
        })
    }
}
