use crate::platform;
use athanor_core::{cancel::Cancellation, features::search::*};
use notify::{RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

enum Change {
    Paths(Vec<PathBuf>),
    Rescan,
    Stop,
}
pub struct FileIndexService {
    sender: mpsc::SyncSender<Change>,
    cancel: Cancellation,
    thread: Option<JoinHandle<()>>,
}
impl FileIndexService {
    pub fn start(
        roots: Vec<PathBuf>,
        cache: PathBuf,
        shared: Arc<RwLock<Arc<SearchSnapshot>>>,
        changed: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let cache = std::path::absolute(&cache).unwrap_or(cache);
        // Canonicalize its parent, including symlinks, before notifications begin.
        let _ = cache.parent().map(std::fs::create_dir_all);
        let cache = cache
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .and_then(|parent| cache.file_name().map(|name| parent.join(name)))
            .unwrap_or(cache);
        let (sender, receiver) = mpsc::sync_channel(16);
        let callback_sender = sender.clone();
        let cancel = Cancellation::default();
        let stop = cancel.clone();
        let overflow = Arc::new(AtomicBool::new(false));
        let thread=std::thread::Builder::new().name("athanor-index".into()).spawn(move ||{
            let roots:Vec<_>=roots.into_iter().map(|p|std::fs::canonicalize(&p).unwrap_or(p)).collect();
            if let Ok(cached)=read_cache(&cache) {
                let entries=cached.entries.into_iter().filter(|e|matches!(e.target,athanor_core::feature::OpenTarget::Application(_))||roots.iter().any(|r|e.location.starts_with(r))).collect();
                *shared.write().unwrap_or_else(|p|p.into_inner())=Arc::new(SearchSnapshot::from_shared(entries));changed();
            }
            let lost=overflow.clone();let cache_event=cache.clone();
            let mut watcher=notify::recommended_watcher(move |event:notify::Result<notify::Event>|{
                let change=match event {
                    Ok(event)=>{
                        if matches!(event.kind,notify::EventKind::Access(_)){return}
                        // Windows reports parent-directory writes when the cache is replaced.
                        // The name index needs directory create/remove/rename events, while
                        // modifications to directory metadata do not change indexed names.
                        let directory_metadata = matches!(
                            event.kind,
                            notify::EventKind::Modify(
                                notify::event::ModifyKind::Any
                                    | notify::event::ModifyKind::Data(_)
                                    | notify::event::ModifyKind::Metadata(_)
                            )
                        );
                        let paths: Vec<_> = event.paths.into_iter().filter(|path| {
                            path != &cache_event
                                && path.file_name().is_none_or(|name| !name.to_string_lossy().starts_with('.'))
                                && !(directory_metadata && path.is_dir())
                        }).collect();
                        if paths.is_empty(){return} Change::Paths(paths)
                    }
                    Err(_)=>Change::Rescan,
                };
                if callback_sender.try_send(change).is_err(){lost.store(true,Ordering::Release);}
            }).ok();
            let mut watched=HashSet::new();
            if let Some(watcher)=watcher.as_mut(){
                for root in roots.iter().chain(platform::app_roots().iter()){
                    if root.is_dir()&&watcher.watch(root,RecursiveMode::NonRecursive).is_ok(){watched.insert(root.clone());}
                }
            }
            let mut parts=HashMap::new();
            let mut apps:Vec<_>=platform::discover_apps().into_iter().map(Arc::new).collect();
            let mut dirty=roots.clone();let mut refresh_apps=false;
            loop {
                if stop.is_cancelled(){return}
                for root in &dirty {
                    if let Ok(part)=build_file_snapshot(std::slice::from_ref(root),&stop){parts.insert(root.clone(),part);}
                }
                if stop.is_cancelled(){return}
                if refresh_apps{apps=platform::discover_apps().into_iter().map(Arc::new).collect();}
                let mut entries=apps.clone();let mut warnings=Vec::new();
                for part in parts.values(){entries.extend(part.entries.iter().cloned());warnings.extend(part.warnings.iter().cloned());}
                let mut snapshot=SearchSnapshot::from_shared(entries);
                snapshot.warnings=warnings;
                if let Some(watcher)=watcher.as_mut(){
                    let mut desired:HashSet<_>=roots.iter().chain(platform::app_roots().iter()).filter(|p|p.is_dir()).cloned().collect();
                    desired.extend(snapshot.entries.iter().filter(|e|e.is_folder).map(|e|e.location.clone()));
                    for path in watched.difference(&desired){let _=watcher.unwatch(path);}
                    watched.retain(|p|desired.contains(p));
                    for path in desired.difference(&watched).cloned().collect::<Vec<_>>(){
                        if watched.len()>=8000{snapshot.warnings.push("Some folders exceed the filesystem watch limit; use Refresh search in settings.".into());break}
                        match watcher.watch(&path,RecursiveMode::NonRecursive){
                            Ok(())=>{watched.insert(path);}
                            Err(_)=>if snapshot.warnings.len()<32{snapshot.warnings.push(format!("Could not watch {} for changes",path.display()));},
                        }
                    }
                }else{snapshot.warnings.push("Filesystem notifications are unavailable; use Refresh search in settings.".into());}
                if write_cache(&cache,&snapshot).is_err(){snapshot.warnings.push("The file-search cache could not be saved.".into());}
                *shared.write().unwrap_or_else(|p|p.into_inner())=Arc::new(snapshot);changed();
                let first=match receiver.recv(){Ok(change)=>change,Err(_)=>return};
                if matches!(first,Change::Stop){return}
                let mut all=overflow.swap(false,Ordering::AcqRel)||matches!(first,Change::Rescan);
                let mut paths=match first{Change::Paths(paths)=>paths,_=>Vec::new()};
                let until=Instant::now()+Duration::from_millis(800);
                while Instant::now()<until{
                    match receiver.recv_timeout(Duration::from_millis(150)){
                        Ok(Change::Stop)=>return,
                        Ok(Change::Rescan)=>all=true,
                        Ok(Change::Paths(mut more))=>{if paths.len()<256{paths.append(&mut more)}else{all=true}},
                        Err(_)=>break,
                    }
                }
                if stop.is_cancelled(){return}
                dirty=if all{roots.clone()}else{roots.iter().filter(|root|paths.iter().any(|path|path.starts_with(root)||root.starts_with(path))).cloned().collect()};
                refresh_apps=all||paths.iter().any(|path|platform::app_roots().iter().any(|root|path.starts_with(root)));
            }
        }).ok();
        Self {
            sender,
            cancel,
            thread,
        }
    }
    pub fn refresh(&self) {
        let _ = self.sender.try_send(Change::Rescan);
    }
}
impl Drop for FileIndexService {
    fn drop(&mut self) {
        self.cancel.cancel();
        let _ = self.sender.try_send(Change::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
