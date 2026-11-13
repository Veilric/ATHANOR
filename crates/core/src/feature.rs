use crate::{
    cancel::Cancellation,
    query::{QueryContext, QueryRequest},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AngleMode {
    #[default]
    Radians,
    Degrees,
}

pub trait Feature: Send + Sync {
    fn id(&self) -> &'static str;
    fn query(
        &self,
        request: &QueryRequest,
        context: &QueryContext,
        cancel: &Cancellation,
    ) -> FeatureReply;
}
pub enum FeatureReply {
    NotApplicable,
    Incomplete,
    Results(Vec<ResultItem>),
    ExclusiveResults(Vec<ResultItem>),
    Invalid { message: String },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultKey {
    pub feature: &'static str,
    pub local: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Presentation {
    Answer,
    Application,
    File,
    Folder,
    Explanation,
    Reference,
}
#[derive(Clone)]
pub struct ResultItem {
    pub key: ResultKey,
    pub title: String,
    pub detail: String,
    pub presentation: Presentation,
    pub actions: Vec<Arc<dyn FeatureAction>>,
}
impl fmt::Debug for ResultItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResultItem")
            .field("key", &self.key)
            .field("title", &self.title)
            .field("detail", &self.detail)
            .field("presentation", &self.presentation)
            .finish_non_exhaustive()
    }
}
impl ResultItem {
    pub fn answer(
        feature: &'static str,
        key: impl Into<String>,
        title: impl Into<String>,
        detail: impl Into<String>,
        copy: impl Into<String>,
    ) -> Self {
        Self {
            key: ResultKey {
                feature,
                local: key.into(),
            },
            title: title.into(),
            detail: detail.into(),
            presentation: Presentation::Answer,
            actions: vec![Arc::new(CopyAction(copy.into()))],
        }
    }
    pub fn explanation(
        feature: &'static str,
        message: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            key: ResultKey {
                feature,
                local: "explanation".into(),
            },
            title: message.into(),
            detail: detail.into(),
            presentation: Presentation::Explanation,
            actions: Vec::new(),
        }
    }
}
pub trait FeatureAction: Send + Sync {
    fn label(&self) -> &str;
    /// Slow native helpers run on a bounded worker; clipboard writes stay on the UI thread.
    fn background(&self) -> bool {
        false
    }
    fn execute(&self, host: &dyn ActionHost) -> Result<ActionOutcome, ActionError>;
}
pub trait ActionHost: Send + Sync {
    fn copy_text(&self, text: &str) -> Result<(), ActionError>;
    fn open(&self, target: &OpenTarget) -> Result<(), ActionError>;
    fn reveal(&self, path: &Path) -> Result<(), ActionError>;
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenTarget {
    Path(#[serde(with = "crate::path_codec")] PathBuf),
    Application(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryInput {
    pub text: String,
    pub submit: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionOutcome {
    pub yield_focus: bool,
    pub input: Option<QueryInput>,
}
impl ActionOutcome {
    pub fn completed(yield_focus: bool) -> Self {
        Self {
            yield_focus,
            input: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionError(pub String);
impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ActionError {}
impl From<std::io::Error> for ActionError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
pub struct CopyAction(pub String);
impl FeatureAction for CopyAction {
    fn label(&self) -> &str {
        "Copy"
    }
    fn execute(&self, host: &dyn ActionHost) -> Result<ActionOutcome, ActionError> {
        host.copy_text(&self.0)?;
        Ok(ActionOutcome::completed(false))
    }
}
pub struct OpenAction(pub OpenTarget);
impl FeatureAction for OpenAction {
    fn background(&self) -> bool {
        true
    }
    fn label(&self) -> &str {
        "Open"
    }
    fn execute(&self, host: &dyn ActionHost) -> Result<ActionOutcome, ActionError> {
        host.open(&self.0)?;
        Ok(ActionOutcome::completed(true))
    }
}
pub struct RevealAction(pub PathBuf);
impl FeatureAction for RevealAction {
    fn background(&self) -> bool {
        true
    }
    fn label(&self) -> &str {
        "Reveal location"
    }
    fn execute(&self, host: &dyn ActionHost) -> Result<ActionOutcome, ActionError> {
        host.reveal(&self.0)?;
        Ok(ActionOutcome::completed(true))
    }
}
