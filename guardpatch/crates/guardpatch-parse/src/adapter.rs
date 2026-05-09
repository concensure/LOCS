use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub id: String,
    pub kind: SymbolKind,
    pub name: String,
    pub start_line: usize,
    pub body_start_line: Option<usize>,
    pub end_line: usize,
    pub is_exported: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Type,
    Enum,
    Module,
    Constant,
    Import,
}

pub trait AstAdapter {
    fn parse_symbols(&self, content: &str) -> anyhow::Result<Vec<SymbolNode>>;
}

pub struct ParserRegistry;

impl ParserRegistry {
    pub fn get_adapter(path: &Path) -> Option<Box<dyn AstAdapter>> {
        let ext = path.extension()?.to_str()?;
        match ext {
            "ts" | "tsx" | "js" | "jsx" => Some(Box::new(crate::typescript::TypeScriptAdapter)),
            "py" => Some(Box::new(crate::python::PythonAdapter)),
            _ => None,
        }
    }
}
