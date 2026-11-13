use super::launch_helper;
use athanor_core::{
    feature::OpenTarget, features::search::SearchEntry, storage::atomic_write_with,
};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};
pub fn app_roots() -> Vec<PathBuf> {
    let mut roots = vec!["/Applications".into(), "/System/Applications".into()];
    if let Some(dirs) = directories::UserDirs::new() {
        roots.push(dirs.home_dir().join("Applications"))
    }
    roots
}
pub fn discover_apps() -> Vec<SearchEntry> {
    let mut apps = Vec::new();
    let mut stack = app_roots();
    while let Some(dir) = stack.pop() {
        let Ok(children) = fs::read_dir(dir) else {
            continue;
        };
        for child in children.flatten() {
            let path = child.path();
            if path.extension().is_some_and(|e| e == "app") {
                let title = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                apps.push(SearchEntry {
                    id: format!("mac:{:?}", path),
                    title,
                    location: path.clone(),
                    target: OpenTarget::Application(format!("mac:{}", path.to_string_lossy())),
                    is_folder: false,
                });
            } else if child
                .file_type()
                .is_ok_and(|t| t.is_dir() && !t.is_symlink())
            {
                stack.push(path)
            }
        }
    }
    apps
}
pub fn open(target: &OpenTarget) -> io::Result<()> {
    match target {
        OpenTarget::Path(path) => launch_helper(
            Command::new("/usr/bin/open").arg("--").arg(path),
            std::time::Duration::from_secs(5),
        ),
        OpenTarget::Application(id) => {
            let path = id
                .strip_prefix("mac:")
                .ok_or_else(|| io::Error::other("Unknown application locator"))?;
            if !Path::new(path).exists() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Application no longer exists",
                ));
            }
            launch_helper(
                Command::new("/usr/bin/open").arg("-a").arg(path),
                std::time::Duration::from_secs(5),
            )
        }
    }
}
pub fn reveal(path: &Path) -> io::Result<()> {
    launch_helper(
        Command::new("/usr/bin/open").arg("-R").arg(path),
        std::time::Duration::from_secs(5),
    )
}
pub fn set_start_at_login(enabled: bool, executable: &Path) -> io::Result<()> {
    let home = directories::UserDirs::new()
        .ok_or_else(|| io::Error::other("Cannot find the home folder"))?;
    let path = home
        .home_dir()
        .join("Library/LaunchAgents/dev.veilric.athanor.plist");
    if !enabled {
        return match fs::remove_file(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            r => r,
        };
    }
    let exe = executable
        .to_string_lossy()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    atomic_write_with(&path, |f| {
        write!(
            f,
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>Label</key><string>dev.veilric.athanor</string><key>ProgramArguments</key><array><string>{exe}</string></array><key>RunAtLoad</key><true/></dict></plist>"
        )
    })
}
