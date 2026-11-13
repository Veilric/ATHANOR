use super::{command_exists, launch, launch_helper};
use athanor_core::{
    feature::OpenTarget, features::search::SearchEntry, storage::atomic_write_with,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

pub fn app_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(dirs) = directories::BaseDirs::new() {
        roots.push(dirs.data_dir().join("applications"));
    }
    let system =
        std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    roots.extend(std::env::split_paths(&system).map(|p| p.join("applications")));
    roots
}
pub fn discover_apps() -> Vec<SearchEntry> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for root in app_roots() {
        let mut stack = vec![root.clone()];
        let mut visited = 0usize;
        while let Some(dir) = stack.pop() {
            let Ok(children) = fs::read_dir(dir) else {
                continue;
            };
            for child in children.flatten() {
                visited += 1;
                if visited > 8000 {
                    break;
                }
                let path = child.path();
                let Ok(kind) = child.file_type() else {
                    continue;
                };
                if kind.is_dir() && !kind.is_symlink() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|x| x != "desktop") {
                    continue;
                }
                let id = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('/', "-");
                if !seen.insert(id) {
                    continue;
                }
                if let Ok(Some(entry)) = read_entry(&path) {
                    apps.push(SearchEntry {
                        id: format!("desktop:{:?}", path.as_os_str()),
                        title: entry.name,
                        location: path.clone(),
                        target: OpenTarget::Application(format!(
                            "desktop:{}",
                            path.to_string_lossy()
                        )),
                        is_folder: false,
                    });
                }
            }
            if visited > 8000 {
                break;
            }
        }
    }
    apps
}
struct DesktopEntry {
    name: String,
    exec: String,
    icon: Option<String>,
    terminal: bool,
}
fn read_entry(path: &Path) -> io::Result<Option<DesktopEntry>> {
    if fs::metadata(path)?.len() > 64 * 1024 {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    let mut fields = HashMap::new();
    let mut in_entry = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if in_entry
            && !line.starts_with('#')
            && let Some((key, value)) = line.split_once('=')
        {
            fields.insert(key.trim(), value.trim());
        }
    }
    if fields.get("Type").copied() != Some("Application")
        || ["Hidden", "NoDisplay"].iter().any(|k| {
            fields
                .get(k)
                .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        })
    {
        return Ok(None);
    }
    let desktops = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let active: Vec<_> = desktops.split(':').collect();
    if fields
        .get("OnlyShowIn")
        .is_some_and(|value| !value.split(';').any(|v| active.contains(&v)))
    {
        return Ok(None);
    }
    if fields.get("NotShowIn").is_some_and(|value| {
        value
            .split(';')
            .any(|v| !v.is_empty() && active.contains(&v))
    }) {
        return Ok(None);
    }
    if fields
        .get("TryExec")
        .is_some_and(|program| !command_exists(program))
    {
        return Ok(None);
    }
    let locale = std::env::var("LC_MESSAGES")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let locale = locale.split('.').next().unwrap_or("");
    let language = locale.split('_').next().unwrap_or("");
    let name = fields
        .get(format!("Name[{locale}]").as_str())
        .or_else(|| fields.get(format!("Name[{language}]").as_str()))
        .or_else(|| fields.get("Name"));
    let Some(name) = name else { return Ok(None) };
    let exec = fields.get("Exec").copied().unwrap_or("");
    if exec.is_empty() && fields.get("DBusActivatable").copied() != Some("true") {
        return Ok(None);
    }
    Ok(Some(DesktopEntry {
        name: name.replace("\\s", " ").replace("\\n", " "),
        exec: exec.into(),
        icon: fields.get("Icon").map(|v| v.to_string()),
        terminal: fields
            .get("Terminal")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
    }))
}
pub fn parse_exec(
    line: &str,
    name: &str,
    icon: Option<&str>,
    desktop: &str,
) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut started = false;
    for ch in line.chars() {
        if escaped {
            word.push(ch);
            escaped = false;
            started = true;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            started = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None
            } else {
                word.push(ch)
            }
            started = true;
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
            started = true
        } else if ch.is_whitespace() {
            if started {
                words.push(std::mem::take(&mut word));
                started = false
            }
        } else {
            word.push(ch);
            started = true
        }
    }
    if quote.is_some() || escaped {
        return Err("Unclosed quote or escape in application command".into());
    }
    if started {
        words.push(word)
    }
    let mut result = Vec::new();
    for word in words {
        if ["%f", "%F", "%u", "%U"].contains(&word.as_str()) {
            continue;
        }
        if word == "%i" {
            if let Some(icon) = icon {
                result.extend(["--icon".into(), icon.into()]);
            }
            continue;
        }
        let mut value = String::new();
        let mut chars = word.chars();
        while let Some(ch) = chars.next() {
            if ch != '%' {
                value.push(ch);
                continue;
            }
            match chars.next() {
                Some('%') => value.push('%'),
                Some('c') => value.push_str(name),
                Some('k') => value.push_str(desktop),
                _ => return Err("Unsupported application field code".into()),
            }
        }
        result.push(value);
    }
    if result.first().is_none_or(String::is_empty) {
        return Err("Application command is empty".into());
    }
    Ok(result)
}
pub fn open(target: &OpenTarget) -> io::Result<()> {
    match target {
        OpenTarget::Path(path) => launch_helper(
            Command::new("xdg-open").arg(path),
            std::time::Duration::from_secs(5),
        ),
        OpenTarget::Application(locator) => {
            let path = locator
                .strip_prefix("desktop:")
                .ok_or_else(|| io::Error::other("Unknown application locator"))?;
            let entry = read_entry(Path::new(path))?
                .ok_or_else(|| io::Error::other("Application entry is unavailable"))?;
            if command_exists("gio") {
                return launch_helper(
                    Command::new("gio").arg("launch").arg(path),
                    std::time::Duration::from_secs(5),
                );
            }
            let args = parse_exec(&entry.exec, &entry.name, entry.icon.as_deref(), path)
                .map_err(io::Error::other)?;
            if entry.terminal {
                let mut command = Command::new("x-terminal-emulator");
                command.arg("-e").args(args);
                launch(&mut command)
            } else {
                launch(Command::new(&args[0]).args(&args[1..]))
            }
        }
    }
}
pub fn reveal(path: &Path) -> io::Result<()> {
    let folder = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    launch_helper(
        Command::new("xdg-open").arg(folder),
        std::time::Duration::from_secs(5),
    )
}
pub fn set_start_at_login(enabled: bool, executable: &Path) -> io::Result<()> {
    let dirs = directories::BaseDirs::new()
        .ok_or_else(|| io::Error::other("Cannot find the configuration folder"))?;
    let path = dirs.config_dir().join("autostart/athanor.desktop");
    if !enabled {
        return match fs::remove_file(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            r => r,
        };
    }
    let mut exe = String::new();
    for ch in executable.to_string_lossy().chars() {
        if ch == '\\' || ch == '"' || ch == '$' || ch as u32 == 96 {
            exe.push('\\');
        }
        exe.push(ch);
    }
    atomic_write_with(&path, |file| {
        write!(
            file,
            "[Desktop Entry]\nType=Application\nName=Athanor\nExec=\"{exe}\"\nTerminal=false\n"
        )
    })
}
