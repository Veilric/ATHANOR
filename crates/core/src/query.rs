use crate::{cancel::Cancellation, feature::*};
use chrono::NaiveDate;
use std::{collections::HashSet, fmt, sync::Arc};

pub const MAX_QUERY_BYTES: usize = 4096;
pub const MAX_RESULTS: usize = 50;
#[derive(Clone, Debug)]
pub struct QueryRequest {
    pub generation: u64,
    pub text: Arc<str>,
}
#[derive(Clone, Debug)]
pub struct QueryContext {
    pub today: NaiveDate,
    pub angle_mode: AngleMode,
}
#[derive(Debug)]
pub struct QueryBatch {
    pub generation: u64,
    pub items: Vec<ResultItem>,
}
impl QueryBatch {
    pub fn can_apply(&self, generation: u64, visible: bool) -> bool {
        visible && generation == self.generation
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct RegistryError(pub &'static str);
impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Duplicate feature identity: {}", self.0)
    }
}
impl std::error::Error for RegistryError {}
pub struct QueryEngine {
    features: Vec<Arc<dyn Feature>>,
}
impl QueryEngine {
    pub fn new(features: Vec<Arc<dyn Feature>>) -> Result<Self, RegistryError> {
        let mut seen = HashSet::new();
        for feature in &features {
            if !seen.insert(feature.id()) {
                return Err(RegistryError(feature.id()));
            }
        }
        Ok(Self { features })
    }
    pub fn query(
        &self,
        request: &QueryRequest,
        context: &QueryContext,
        cancel: &Cancellation,
    ) -> QueryBatch {
        let mut batch = QueryBatch {
            generation: request.generation,
            items: Vec::new(),
        };
        if request.text.trim().is_empty() || cancel.is_cancelled() {
            return batch;
        }
        if request.text.len() > MAX_QUERY_BYTES {
            batch.items.push(ResultItem::explanation(
                "query",
                "Query is too long",
                "Use at most 4,096 bytes.",
            ));
            return batch;
        }
        let mut grouped = Vec::new();
        for feature in &self.features {
            if cancel.is_cancelled() {
                batch.items.clear();
                return batch;
            }
            match feature.query(request, context, cancel) {
                FeatureReply::ExclusiveResults(items) => {
                    if !cancel.is_cancelled() {
                        batch.items = items.into_iter().take(MAX_RESULTS).collect();
                    }
                    return batch;
                }
                FeatureReply::Results(items) => {
                    let priority = if items
                        .iter()
                        .any(|item| item.presentation == Presentation::Answer)
                    {
                        0
                    } else if items
                        .iter()
                        .any(|item| item.presentation == Presentation::Explanation)
                    {
                        1
                    } else {
                        2
                    };
                    grouped.extend(items.into_iter().map(|item| (priority, item)));
                }
                FeatureReply::Invalid { message } => grouped.push((
                    1,
                    ResultItem::explanation(
                        feature.id(),
                        message,
                        "Check the query and try again.",
                    ),
                )),
                FeatureReply::NotApplicable | FeatureReply::Incomplete => {}
            }
        }
        if cancel.is_cancelled() {
            batch.items.clear();
            return batch;
        }
        // Stable sorting preserves a feature's own ranking and registration order.
        grouped.sort_by_key(|(priority, _)| *priority);
        batch.items = grouped
            .into_iter()
            .take(MAX_RESULTS)
            .map(|(_, item)| item)
            .collect();
        batch
    }
}
