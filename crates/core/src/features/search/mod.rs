mod index;
mod rank;
use crate::{cancel::Cancellation, feature::*, query::*};
pub use index::{build_file_snapshot, read_cache, write_cache};
use serde::{Deserialize, Serialize};
use std::{
    collections::BinaryHeap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchEntry {
    pub id: String,
    pub title: String,
    #[serde(with = "crate::path_codec")]
    pub location: PathBuf,
    pub target: OpenTarget,
    pub is_folder: bool,
}
#[derive(Default, Debug)]
pub struct SearchSnapshot {
    pub entries: Vec<Arc<SearchEntry>>,
    pub warnings: Vec<String>,
    folded: Vec<String>,
}
impl SearchSnapshot {
    pub fn new(entries: Vec<SearchEntry>) -> Self {
        Self::from_shared(entries.into_iter().map(Arc::new).collect())
    }
    pub fn from_shared(mut entries: Vec<Arc<SearchEntry>>) -> Self {
        entries.sort_by_cached_key(|entry| (entry.title.to_lowercase(), entry.id.clone()));
        let folded = entries
            .iter()
            .map(|entry| entry.title.to_lowercase())
            .collect();
        Self {
            entries,
            folded,
            warnings: Vec::new(),
        }
    }
}
pub struct SearchFeature {
    snapshot: Arc<RwLock<Arc<SearchSnapshot>>>,
}
impl SearchFeature {
    pub fn new(snapshot: Arc<RwLock<Arc<SearchSnapshot>>>) -> Self {
        Self { snapshot }
    }
}
impl Feature for SearchFeature {
    fn id(&self) -> &'static str {
        "search"
    }
    fn query(
        &self,
        request: &QueryRequest,
        _: &QueryContext,
        cancel: &Cancellation,
    ) -> FeatureReply {
        let query = request.text.trim().to_lowercase();
        if query.is_empty() || cancel.is_cancelled() {
            return FeatureReply::NotApplicable;
        }
        let snapshot = self
            .snapshot
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let mut best = BinaryHeap::with_capacity(MAX_RESULTS + 1);
        for (i, name) in snapshot.folded.iter().enumerate() {
            if i % 256 == 0 && cancel.is_cancelled() {
                return FeatureReply::NotApplicable;
            }
            if let Some(score) = rank::rank(name, &query) {
                best.push((score, i));
                if best.len() > MAX_RESULTS {
                    best.pop();
                }
            }
        }
        let mut selected = best.into_vec();
        selected.sort_unstable();
        let items = selected
            .into_iter()
            .map(|(_, i)| {
                let entry = &snapshot.entries[i];
                let mut actions: Vec<Arc<dyn FeatureAction>> =
                    vec![Arc::new(OpenAction(entry.target.clone()))];
                let presentation = match entry.target {
                    OpenTarget::Application(_) => Presentation::Application,
                    OpenTarget::Path(_) if entry.is_folder => Presentation::Folder,
                    OpenTarget::Path(_) => Presentation::File,
                };
                if matches!(entry.target, OpenTarget::Path(_)) {
                    actions.push(Arc::new(CopyAction(
                        entry.location.to_string_lossy().into_owned(),
                    )));
                    actions.push(Arc::new(RevealAction(entry.location.clone())));
                }
                ResultItem {
                    key: ResultKey {
                        feature: self.id(),
                        local: entry.id.clone(),
                    },
                    title: entry.title.clone(),
                    detail: entry.location.to_string_lossy().into_owned(),
                    presentation,
                    actions,
                }
            })
            .collect();
        FeatureReply::Results(items)
    }
}
