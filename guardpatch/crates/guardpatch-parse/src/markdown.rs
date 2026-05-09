use pulldown_cmark::{Event, Parser, Tag};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Per-document section governance declared via HTML comments inside a Markdown file:
/// `<!-- guardpatch-locked: Title, Core Principles -->`
/// `<!-- guardpatch-editable: Examples, Changelog -->`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InlineMarkdownPolicy {
    pub locked: Vec<String>,
    pub editable: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSection {
    pub title: String,
    pub level: u32,
    pub start_line: usize,
    pub end_line: usize,
}

pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse_sections(content: &str) -> Vec<DocumentSection> {
        let parser = Parser::new(content);
        let mut sections = Vec::new();
        let mut current_heading: Option<(String, u32, usize)> = None;

        let lines: Vec<&str> = content.lines().collect();

        for (event, range) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    let start_line = content[..range.start].lines().count();
                    
                    if let Some((title, lvl, start)) = current_heading.take() {
                        sections.push(DocumentSection {
                            title,
                            level: lvl,
                            start_line: start,
                            end_line: start_line.max(start),
                        });
                    }
                    current_heading = Some((String::new(), level as u32, start_line));
                }
                Event::Text(text) => {
                    if let Some((ref mut title, _, _)) = current_heading {
                        if !title.is_empty() {
                            title.push(' ');
                        }
                        title.push_str(&text);
                    }
                }
                _ => {}
            }
        }

        if let Some((title, level, start)) = current_heading {
            sections.push(DocumentSection {
                title,
                level,
                start_line: start,
                end_line: lines.len(),
            });
        }

        sections
    }

    /// Parse per-document inline governance declared as HTML comments.
    ///
    /// Supported syntax (case-insensitive):
    /// ```markdown
    /// <!-- guardpatch-locked: Title, Core Principles, Metadata Schema -->
    /// <!-- guardpatch-editable: Examples, Implementation Notes, Changelog -->
    /// ```
    pub fn parse_inline_policy(content: &str) -> InlineMarkdownPolicy {
        let mut policy = InlineMarkdownPolicy::default();

        if let Ok(re) = Regex::new(r"(?i)<!--\s*guardpatch-locked:\s*([^>-][^>]*?)\s*-->") {
            if let Some(caps) = re.captures(content) {
                if let Some(m) = caps.get(1) {
                    policy.locked = m.as_str()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }

        if let Ok(re) = Regex::new(r"(?i)<!--\s*guardpatch-editable:\s*([^>-][^>]*?)\s*-->") {
            if let Some(caps) = re.captures(content) {
                if let Some(m) = caps.get(1) {
                    policy.editable = m.as_str()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }

        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inline_policy_locked_and_editable() {
        let content = r#"# Doc
<!-- guardpatch-locked: Title, Core Principles, Metadata Schema -->
<!-- guardpatch-editable: Examples, Changelog -->
## Title
"#;
        let policy = MarkdownParser::parse_inline_policy(content);
        assert!(policy.locked.contains(&"Title".to_string()));
        assert!(policy.locked.contains(&"Core Principles".to_string()));
        assert!(policy.locked.contains(&"Metadata Schema".to_string()));
        assert!(policy.editable.contains(&"Examples".to_string()));
        assert!(policy.editable.contains(&"Changelog".to_string()));
    }

    #[test]
    fn test_parse_inline_policy_empty() {
        let content = "# No policy here\n\nJust some text.\n";
        let policy = MarkdownParser::parse_inline_policy(content);
        assert!(policy.locked.is_empty());
        assert!(policy.editable.is_empty());
    }
}
