use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum StructuredOperation {
    ReplaceFile {
        file: PathBuf,
        content: String,
    },
    ReplaceSymbolBody {
        file: PathBuf,
        symbol_name: String,
        body: String,
    },
    AppendSection {
        file: PathBuf,
        section_title: String,
        content: String,
    },
    InsertAfterMarker {
        file: PathBuf,
        marker_id: String,
        content: String,
    },
    CreateFile {
        file: PathBuf,
        content: String,
    },
    DeleteFile {
        file: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredPatch {
    pub operations: Vec<StructuredOperation>,
}

impl StructuredPatch {
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}
