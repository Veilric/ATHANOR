//! One process per data directory. IPC is loopback-only and carries no queries.
use athanor_core::storage::atomic_write_with;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceCommand {
    Show,
    Settings,
    Quit,
}
impl InstanceCommand {
    fn name(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Settings => "settings",
            Self::Quit => "quit",
        }
    }
    fn parse(value: &str) -> Option<Self> {
        match value {
            "show" => Some(Self::Show),
            "settings" => Some(Self::Settings),
            "quit" => Some(Self::Quit),
            _ => None,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Endpoint {
    port: u16,
    token: String,
}
pub type WakeSlot = Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>;
pub struct InstanceServer {
    _lock: File,
    endpoint: PathBuf,
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub wake: WakeSlot,
}
pub enum Start {
    Primary(InstanceServer, mpsc::Receiver<InstanceCommand>),
    Forwarded,
}
impl InstanceServer {
    pub fn start(directory: &Path) -> io::Result<Start> {
        fs::create_dir_all(directory)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("instance.lock"))?;
        if lock.try_lock().is_err() {
            for _ in 0..20 {
                if send_existing(directory, InstanceCommand::Show).is_ok() {
                    return Ok(Start::Forwarded);
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            return Err(io::Error::other(
                "Athanor is already running, but its local command endpoint is not responding",
            ));
        }
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = listener.local_addr()?;
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|e| io::Error::other(e.to_string()))?;
        let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let endpoint = directory.join("instance.json");
        atomic_write_with(&endpoint, |file| {
            serde_json::to_writer(
                file,
                &Endpoint {
                    port: address.port(),
                    token: token.clone(),
                },
            )
            .map_err(io::Error::other)
        })?;
        let (sender, receiver) = mpsc::sync_channel(16);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let wake: WakeSlot = Arc::new(Mutex::new(None));
        let wake_thread = wake.clone();
        let thread = std::thread::Builder::new()
            .name("athanor-instance".into())
            .spawn(move || {
                for client in listener.incoming() {
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(mut stream) = client else { continue };
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
                    let mut line = String::new();
                    let Ok(clone) = stream.try_clone() else {
                        continue;
                    };
                    if BufReader::new(clone)
                        .take(128)
                        .read_line(&mut line)
                        .is_err()
                    {
                        continue;
                    }
                    let mut parts = line.split_whitespace();
                    if parts.next() != Some(token.as_str()) {
                        continue;
                    }
                    let Some(command) = parts.next().and_then(InstanceCommand::parse) else {
                        continue;
                    };
                    if parts.next().is_some() {
                        continue;
                    }
                    if sender.try_send(command).is_ok() {
                        let _ = stream.write_all(b"ok\n");
                        if let Some(wake) = wake_thread
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .as_ref()
                        {
                            wake();
                        }
                    }
                }
            })?;
        Ok(Start::Primary(
            Self {
                _lock: lock,
                endpoint,
                address,
                stop,
                thread: Some(thread),
                wake,
            },
            receiver,
        ))
    }
}
pub fn send_existing(directory: &Path, command: InstanceCommand) -> io::Result<()> {
    let path = directory.join("instance.json");
    if fs::metadata(&path)?.len() > 1024 {
        return Err(io::Error::other("Invalid local command endpoint"));
    }
    let endpoint: Endpoint = serde_json::from_slice(&fs::read(path)?)?;
    if endpoint.token.len() != 32 || !endpoint.token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(io::Error::other("Invalid local command endpoint"));
    }
    let address = SocketAddr::from(([127, 0, 0, 1], endpoint.port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(300))?;
    stream.set_read_timeout(Some(Duration::from_millis(300)))?;
    stream.write_all(format!("{} {}\n", endpoint.token, command.name()).as_bytes())?;
    let mut reply = String::new();
    BufReader::new(stream).take(32).read_line(&mut reply)?;
    if reply.trim() == "ok" {
        Ok(())
    } else {
        Err(io::Error::other(
            "The running instance did not accept the command",
        ))
    }
}
impl Drop for InstanceServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_millis(100));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = fs::remove_file(&self.endpoint);
    }
}
