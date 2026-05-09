use tree_sitter::{Parser, Query, QueryCursor};
use crate::adapter::{AstAdapter, SymbolNode, SymbolKind};

pub struct TypeScriptAdapter;

impl AstAdapter for TypeScriptAdapter {
    fn parse_symbols(&self, content: &str) -> anyhow::Result<Vec<SymbolNode>> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;
        
        let tree = parser.parse(content, None).ok_or_else(|| anyhow::anyhow!("Failed to parse TS"))?;
        let root_node = tree.root_node();

        // Query to find interesting symbols
        let query_str = r#"
            (function_declaration name: (identifier) @name) @function
            (class_declaration name: (identifier) @name) @class
            (method_definition name: (property_identifier) @name) @method
            (interface_declaration name: (type_identifier) @name) @interface
            (type_alias_declaration name: (type_identifier) @name) @type
            (lexical_declaration (variable_declarator name: (identifier) @name value: (arrow_function))) @arrow_func
            (import_statement) @import
        "#;

        let query = Query::new(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), query_str)?;
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
                2 => kind = SymbolKind::Method,
                3 => kind = SymbolKind::Interface,
                4 => kind = SymbolKind::Type,
                5 => kind = SymbolKind::Function,
                6 => kind = SymbolKind::Import,
                _ => {}
            }

            // For imports, we might not have a simple "name" capture in the same way
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
            
            // Re-capture the main node for range
            let main_node = m.nodes_for_capture_index(0).next().unwrap();

            symbols.push(SymbolNode {
                id: format!("{}:{}", name, kind as u32), // Simple ID for now
                kind,
                name: name.clone(),
                start_line: main_node.start_position().row + 1,
                body_start_line: Some(main_node.start_position().row + 2), // Rough heuristic
                end_line: main_node.end_position().row + 1,
                is_exported: false, // TODO: Check for export modifier
            });
        }

        Ok(symbols)
    }
}
