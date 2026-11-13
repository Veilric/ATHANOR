pub mod capabilities;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(feature = "native-ui")]
use athanor_core::feature::*;
use athanor_core::features::search::SearchEntry;
use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(target_os = "linux")]
use linux as native;
#[cfg(target_os = "macos")]
use macos as native;
#[cfg(target_os = "windows")]
use windows as native;
pub fn discover_apps() -> Vec<SearchEntry> {
    native::discover_apps()
}
pub fn app_roots() -> Vec<PathBuf> {
    native::app_roots()
}
pub fn set_start_at_login(enabled: bool, executable: &Path) -> io::Result<()> {
    native::set_start_at_login(enabled, executable)
}
#[cfg(target_os = "linux")]
pub(crate) fn launch(command: &mut Command) -> io::Result<()> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let _ = std::thread::Builder::new()
        .name("athanor-launch".into())
        .spawn(move || {
            let _ = child.wait();
        });
    Ok(())
}
/// Confirm short-lived OS opener helpers, without blocking the UI or hiding failures.
pub fn launch_helper(command: &mut Command, timeout: std::time::Duration) -> io::Result<()> {
    let name = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "{name} rejected the target ({status}). Check that it exists and an associated app is installed."
                )))
            };
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(
                "The opener did not confirm success in time. Check the destination app before trying again.",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[cfg(target_os = "linux")]
pub(crate) fn command_exists(command: &str) -> bool {
    if command.contains(std::path::MAIN_SEPARATOR) {
        return Path::new(command).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(command).is_file()))
}
#[cfg(feature = "native-ui")]
#[derive(Default)]
pub struct NativeActionHost {
    clipboard: std::sync::Mutex<Option<arboard::Clipboard>>,
}
#[cfg(feature = "native-ui")]
impl ActionHost for NativeActionHost {
    fn copy_text(&self, text: &str) -> Result<(), ActionError> {
        let mut clipboard = self
            .clipboard
            .lock()
            .map_err(|_| ActionError("Clipboard is unavailable".into()))?;
        if clipboard.is_none() {
            *clipboard = Some(
                arboard::Clipboard::new()
                    .map_err(|_| ActionError("Clipboard is unavailable".into()))?,
            );
        }
        clipboard
            .as_mut()
            .ok_or_else(|| ActionError("Clipboard is unavailable".into()))?
            .set_text(text)
            .map_err(|_| ActionError("Could not copy the result".into()))
    }
    fn open(&self, target: &OpenTarget) -> Result<(), ActionError> {
        if let OpenTarget::Path(path) = target
            && !path.exists()
        {
            return Err(ActionError("That file or folder no longer exists".into()));
        }
        native::open(target).map_err(Into::into)
    }
    fn reveal(&self, path: &Path) -> Result<(), ActionError> {
        if !path.exists() {
            return Err(ActionError("That file or folder no longer exists".into()));
        }
        native::reveal(path).map_err(Into::into)
    }
}
