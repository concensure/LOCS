use tree_sitter::{Parser, Query, QueryCursor};
use crate::adapter::{AstAdapter, SymbolNode, SymbolKind};

pub struct PythonAdapter;

impl AstAdapter for PythonAdapter {
    fn parse_symbols(&self, content: &str) -> anyhow::Result<Vec<SymbolNode>> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())?;
        
        let tree = parser.parse(content, None).ok_or_else(|| anyhow::anyhow!("Failed to parse Python"))?;
        let root_node = tree.root_node();

        let query_str = r#"
            (function_definition name: (identifier) @name) @function
            (class_definition name: (identifier) @name) @class
            (import_statement) @import
            (import_from_statement) @import
        "#;

        let query = Query::new(&tree_sitter_python::LANGUAGE.into(), query_str)?;
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, root_node, content.as_bytes());

        let mut symbols = Vec::new();

        for m in matches {
            let mut name = String::new();
            let mut kind = SymbolKind::Function;
            
            let pattern_index = m.pattern_index;
            match pattern_index {
                0 => kind = SymbolKind::Function,
                1 => kind = SymbolKind::Class,
                2 | 3 => kind = SymbolKind::Import,
                _ => {}
            }

            if kind == SymbolKind::Import {
                let node = m.nodes_for_capture_index(0).next().unwrap();
                name = node.utf8_text(content.as_bytes())?.to_string();
            } else {
                for capture in m.captures {
                    if query.capture_names()[capture.index as usize] == "name" {
                        name = capture.node.utf8_text(content.as_bytes())?.to_string();
                    }
                }
            }
            
            let main_node = m.nodes_for_capture_index(0).next().unwrap();

            symbols.push(SymbolNode {
                id: format!("{}:{}", name, kind as u32),
                kind,
                name: name.clone(),
                start_line: main_node.start_position().row + 1,
                body_start_line: Some(main_node.start_position().row + 2),
                end_line: main_node.end_position().row + 1,
                is_exported: true, // Python "exports" everything by default
            });
        }

        Ok(symbols)
    }
}
