#![cfg_attr(
    all(target_os = "windows", feature = "native-ui"),
    windows_subsystem = "windows"
)]
use athanor::{
    bootstrap::{self, Services},
    config::{AppPaths, Config},
};
use athanor_core::query::QueryRequest;
use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Athanor: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--quit") {
        let paths = AppPaths::discover().map_err(|e| e.to_string())?;
        return athanor::services::instance::send_existing(
            &paths.data,
            athanor::services::instance::InstanceCommand::Quit,
        )
        .map_err(|e| e.to_string());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "Athanor\n\nStart without arguments to run the native launcher in the background.\n\n  --show                    Start and show the panel\n  --quit                    Quit the running instance\n  --query EXPRESSION --json  Evaluate a query without opening the UI\n  --capabilities            Show unverified platform capability policy\n  --list-scenes             List native UI test scenes\n  --snap PATH --scene NAME  Render a deterministic UI scene\n\nDefault shortcut: Ctrl+Alt+Space (Option+Space on macOS).\nExamples: sqrt(144)+2^5; 12 km in mi; today + 45 days; 100 usd to php."
        );
        return Ok(());
    }
    if let Some(index) = args.iter().position(|a| a == "--query") {
        let query = args.get(index + 1).ok_or("Pass a query after --query")?;
        let paths = AppPaths::discover().map_err(|e| e.to_string())?;
        let (config, _) = Config::load(&paths.config);
        let services = Services::real(&config, &paths, Arc::new(|| {}));
        let request = QueryRequest {
            generation: 1,
            text: Arc::from(query.as_str()),
        };
        let context = bootstrap::context(&config, false);
        let mut batch = services
            .engine
            .query(&request, &context, &Default::default());
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut revision = 0;
        while !batch.items.iter().any(|item| !item.actions.is_empty()) && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(15));
            let next = services.revision.load(Ordering::Acquire);
            if next != revision {
                revision = next;
                batch = services
                    .engine
                    .query(&request, &context, &Default::default());
            }
        }
        if args.iter().any(|a| a == "--json") {
            let items:Vec<_>=batch.items.iter().map(|item|serde_json::json!({
                "feature":item.key.feature,"title":item.title,"detail":item.detail,"kind":item.presentation,
                "actions":item.actions.iter().map(|a|a.label()).collect::<Vec<_>>()
            })).collect();
            println!(
                "{}",
                serde_json::json!({"generation":batch.generation,"items":items})
            );
        } else {
            for item in &batch.items {
                println!("{}\n  {}", item.title, item.detail);
            }
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--capabilities") {
        use athanor::platform::capabilities::{CapabilityReport, PanelBackend};
        let backend = if cfg!(target_os = "macos") {
            PanelBackend::MacOSPanel
        } else if cfg!(target_os = "windows") {
            PanelBackend::WindowsWindow
        } else if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            PanelBackend::X11Window
        } else {
            PanelBackend::Unsupported
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&CapabilityReport::candidate(backend))
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    #[cfg(feature = "native-ui")]
    return athanor::ui::native::run();
    #[cfg(not(feature = "native-ui"))]
    Err("This build has no native UI. Rebuild with the native-ui feature.".into())
}
