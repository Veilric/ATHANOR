use athanor_core::{feature::OpenTarget, features::search::SearchEntry};
use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
pub fn app_roots() -> Vec<PathBuf> {
    ["APPDATA", "PROGRAMDATA"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(|p| PathBuf::from(p).join("Microsoft/Windows/Start Menu/Programs"))
        .collect()
}
pub fn discover_apps() -> Vec<SearchEntry> {
    let script = "[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); Get-StartApps | Select-Object Name,AppID | ConvertTo-Json -Compress";
    let Ok(output) = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return vec![];
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return vec![];
    };
    let values = match value {
        serde_json::Value::Array(v) => v,
        v => vec![v],
    };
    values
        .into_iter()
        .filter_map(|v| {
            let name = v["Name"].as_str()?.to_owned();
            let id = v["AppID"].as_str()?.to_owned();
            Some(SearchEntry {
                id: format!("windows:{id}"),
                title: name,
                location: PathBuf::from(&id),
                target: OpenTarget::Application(format!("windows:{id}")),
                is_folder: false,
            })
        })
        .collect()
}
pub fn open(target: &OpenTarget) -> io::Result<()> {
    match target {
        OpenTarget::Path(path) => shell_open(path.as_os_str(), None),
        OpenTarget::Application(id) => {
            let id = id
                .strip_prefix("windows:")
                .ok_or_else(|| io::Error::other("Unknown application locator"))?;
            shell_open(
                std::ffi::OsStr::new(&format!("shell:AppsFolder\\{id}")),
                None,
            )
        }
    }
}
pub fn reveal(path: &Path) -> io::Result<()> {
    let mut parameters = std::ffi::OsString::from("/select,\"");
    parameters.push(path);
    parameters.push("\"");
    shell_open(std::ffi::OsStr::new("explorer.exe"), Some(&parameters))
}
fn shell_open(target: &std::ffi::OsStr, parameters: Option<&std::ffi::OsStr>) -> io::Result<()> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            window: *mut c_void,
            verb: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }
    fn wide(value: &std::ffi::OsStr) -> io::Result<Vec<u16>> {
        let mut units: Vec<_> = value.encode_wide().collect();
        if units.contains(&0) {
            return Err(io::Error::other(
                "The target contains an invalid NUL character",
            ));
        }
        units.push(0);
        Ok(units)
    }
    let target = wide(target)?;
    let parameters = parameters.map(wide).transpose()?;
    let verb: Vec<_> = "open\0".encode_utf16().collect();
    let code = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            parameters.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            std::ptr::null(),
            1,
        )
    };
    if code <= 32 {
        Err(io::Error::other(format!(
            "Windows could not open this target (ShellExecute error {code}). Check the target and its file association."
        )))
    } else {
        Ok(())
    }
}
pub fn set_start_at_login(enabled: bool, executable: &Path) -> io::Result<()> {
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let mut command = Command::new("reg.exe");
    if enabled {
        command
            .args(["add", key, "/v", "Athanor", "/t", "REG_SZ", "/d"])
            .arg(format!("\"{}\"", executable.display()))
            .arg("/f");
    } else {
        command.args(["delete", key, "/v", "Athanor", "/f"]);
    }
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() || !enabled {
        Ok(())
    } else {
        Err(io::Error::other("Could not update start-at-login settings"))
    }
}
