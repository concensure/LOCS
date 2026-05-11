use guardpatch_policy::{GuardMode, SectionRole};
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
    pub id: Option<String>,
    pub mode: Option<GuardMode>,
    pub role: Option<SectionRole>,
    pub level: u32,
    pub start_line: usize,
    pub end_line: usize,
}

pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse_sections(content: &str) -> Vec<DocumentSection> {
        let parser = Parser::new(content);
        let mut sections = Vec::new();
        let mut current_heading: Option<(String, u32, usize, Option<String>, Option<GuardMode>, Option<SectionRole>)> = None;

        let lines: Vec<&str> = content.lines().collect();

        // Regex to match locs anchors: <!-- locs:id=... locs:edit=... locs:role=... -->
        let re_anchor = Regex::new(r"<!--\s*locs:(?:id=([^\s>]+))?\s*(?:locs:edit=([^\s>]+))?\s*(?:locs:role=([^\s>]+))?\s*-->").unwrap();

        for (event, range) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    let start_line = content[..range.start].lines().count();
                    
                    if let Some((title, lvl, start, id, mode, role)) = current_heading.take() {
                        sections.push(DocumentSection {
                            title,
                            id,
                            mode,
                            role,
                            level: lvl,
                            start_line: start,
                            end_line: start_line.max(start),
                        });
                    }

                    // Look for anchor in the heading line
                    let heading_text = &content[range.start..range.end];
                    let mut id = None;
                    let mut mode = None;
                    let mut role = None;

                    if let Some(caps) = re_anchor.captures(heading_text) {
                        id = caps.get(1).map(|m| m.as_str().to_string());
                        mode = caps.get(2).and_then(|m| match m.as_str() {
                            "locked" | "protected" => Some(GuardMode::Protected),
                            "editable" => Some(GuardMode::Editable),
                            "approval" | "review_required" => Some(GuardMode::ReviewRequired),
                            _ => None,
                        });
                        role = caps.get(3).and_then(|m| match m.as_str() {
                            "metadata" => Some(SectionRole::Metadata),
                            "contract" => Some(SectionRole::Contract),
                            "implementation" => Some(SectionRole::Implementation),
                            "example" => Some(SectionRole::Example),
                            "notes" => Some(SectionRole::Notes),
                            _ => None,
                        });
                    }

                    current_heading = Some((String::new(), level as u32, start_line, id, mode, role));
                }
                Event::Text(text) => {
                    if let Some((ref mut title, _, _, _, _, _)) = current_heading {
                        // Skip the anchor comment in the title text
                        let cleaned_text = re_anchor.replace(&text, "").to_string();
                        if !cleaned_text.trim().is_empty() {
                            if !title.is_empty() {
                                title.push(' ');
                            }
                            title.push_str(cleaned_text.trim());
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some((title, level, start, id, mode, role)) = current_heading {
            sections.push(DocumentSection {
                title,
                id,
                mode,
                role,
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
