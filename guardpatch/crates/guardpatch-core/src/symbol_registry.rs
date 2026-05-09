use std::collections::HashMap;
use std::path::PathBuf;
use guardpatch_parse::SymbolNode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub file: PathBuf,
    pub symbol: SymbolNode,
}

pub struct SymbolRegistry {
    // Map fully qualified name (or ID) to entry
    pub symbols: HashMap<String, RegistryEntry>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    pub fn register_file_symbols(&mut self, file: PathBuf, symbols: Vec<SymbolNode>) {
        for symbol in symbols {
            // Simple FQN: file_path::symbol_name
            let fqn = format!("{}::{}", file.display(), symbol.name);
            self.symbols.insert(fqn, RegistryEntry {
                file: file.clone(),
                symbol,
            });
        }
    }

    pub fn lookup(&self, fqn: &str) -> Option<&RegistryEntry> {
        self.symbols.get(fqn)
    }

    pub fn find_by_range(&self, file: &PathBuf, line: usize) -> Option<&SymbolNode> {
        self.symbols.values()
            .filter(|e| &e.file == file)
            .map(|e| &e.symbol)
            .find(|s| line >= s.start_line && line <= s.end_line)
    }
}
