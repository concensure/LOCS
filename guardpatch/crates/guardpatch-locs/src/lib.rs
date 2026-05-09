use guardpatch_policy::GuardMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocsMetadata {
    pub capability: Option<String>,
    pub stability: Option<String>,
    pub owner: Option<String>,
    pub kind: Option<String>,
    pub guard: Option<GuardConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuardConfig {
    pub mode: Option<GuardMode>,
    pub lock_signature: Option<bool>,
    pub lock_body: Option<bool>,
    pub require_tests: Option<bool>,
    /// Named regions that are locked: "metadata", "public-interface", "behaviour-contract",
    /// "implementation", "internal-helpers", or any Markdown section title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_regions: Option<Vec<String>>,
    /// Named regions explicitly editable — overrides file-level protection for these regions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editable_regions: Option<Vec<String>>,
    /// Change types that require human approval: "interface-change", "dependency-change", "state-change".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_required: Option<Vec<String>>,
    /// Evidence kinds that must pass before an edit is accepted: "tests-pass", "typecheck-pass".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_required: Option<Vec<String>>,
}

pub struct LocsExtractor;

impl LocsExtractor {
    /// Return the (start_line, end_line) of the LOCS metadata block within `content`,
    /// covering YAML frontmatter (`--- ... ---`) or a block comment (`/* LOCS: ... */`).
    pub fn find_metadata_line_range(content: &str) -> Option<(usize, usize)> {
        // YAML frontmatter: file starts with --- ... ---
        if content.starts_with("---") {
            let after = &content[3..];
            if let Some(close_offset) = after.find("\n---") {
                let end_line = content[..3 + close_offset + 4].lines().count();
                return Some((0, end_line));
            }
        }

        // Block comment containing LOCS:
        let lines: Vec<&str> = content.lines().collect();
        let mut block_start: Option<usize> = None;
        let mut saw_locs = false;

        for (i, line) in lines.iter().enumerate() {
            let t = line.trim_start();
            if block_start.is_none() && t.starts_with("/*") {
                block_start = Some(i);
                saw_locs = false;
            }
            if block_start.is_some() && t.contains("LOCS:") {
                saw_locs = true;
            }
            if block_start.is_some() && (t.ends_with("*/") || t == "*/") {
                if saw_locs {
                    return Some((block_start.unwrap(), i));
                }
                block_start = None;
                saw_locs = false;
            }
        }
        None
    }

    pub fn extract_from_markdown(content: &str) -> Option<LocsMetadata> {
        // Simple frontmatter extractor: looks for --- ... --- at the start
        if !content.starts_with("---") {
            return None;
        }

        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }

        let yaml = parts[1];
        serde_yaml::from_str::<LocsMetadata>(yaml).ok()
    }

    pub fn extract_from_comments(content: &str) -> Option<LocsMetadata> {
        // Block comment: /* ... LOCS: \n <yaml> */
        let re_block = regex::Regex::new(r"(?s)LOCS:\s*\n(.*?)\*/").ok()?;
        if let Some(caps) = re_block.captures(content) {
            if let Some(block) = caps.get(1) {
                let cleaned = Self::clean_comment_lines(block.as_str());
                if let Ok(meta) = serde_yaml::from_str::<LocsMetadata>(&cleaned) {
                    return Some(meta);
                }
            }
        }

        // Line comments: // LOCS: or # LOCS: followed by indented lines
        let re_line = regex::Regex::new(r"(?m)^[ \t]*(?://|#)\s*LOCS:\s*\n((?:[ \t]*(?://|#)[^\n]*\n)*)").ok()?;
        if let Some(caps) = re_line.captures(content) {
            if let Some(block) = caps.get(1) {
                let cleaned = Self::clean_comment_lines(block.as_str());
                if let Ok(meta) = serde_yaml::from_str::<LocsMetadata>(&cleaned) {
                    return Some(meta);
                }
            }
        }

        None
    }

    fn clean_comment_lines(block: &str) -> String {
        let lines: Vec<&str> = block.lines().collect();

        // Step 1: strip a uniform comment-prefix per line if present (e.g. " * ").
        let stripped: Vec<&str> = lines.iter().map(|line| {
            let t = line.trim_start();
            // Strip a single leading comment marker followed by optional space.
            let t = if t.starts_with("* ") || t.starts_with("* ") {
                t.trim_start_matches('*').trim_start_matches(' ')
            } else if t.starts_with("// ") {
                t.trim_start_matches('/').trim_start_matches(' ')
            } else if t.starts_with("# ") {
                t.trim_start_matches('#').trim_start_matches(' ')
            } else {
                // No comment marker — use raw line (preserve indentation from original)
                line
            };
            t
        }).collect();

        // Step 2: find the minimum indentation among non-empty stripped lines.
        let min_indent = stripped.iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);

        // Step 3: strip the uniform indent to produce clean YAML.
        stripped.iter()
            .map(|line| {
                if line.len() >= min_indent { &line[min_indent..] } else { line.trim_start() }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl LocsMetadata {
    pub fn template_markdown(capability: &str) -> String {
        format!(
            "---\nlocs:\n  capability: {}\n  stability: draft\n  owner: core\nguard:\n  mode: editable\n---\n\n# Document Title\n",
            capability
        )
    }

    pub fn template_rust(capability: &str) -> String {
        format!(
            "/*\nLOCS:\n  capability: {}\n  stability: draft\n  owner: core\n  kind: source\nguard:\n  mode: editable\n*/\n\npub fn placeholder() {{\n}}\n",
            capability
        )
    }

    pub fn template_python(capability: &str) -> String {
        format!(
            "\"\"\"\nLOCS:\n  capability: {}\n  stability: draft\n  owner: core\n  kind: source\nguard:\n  mode: editable\n\"\"\"\n\ndef placeholder():\n    pass\n",
            capability
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_markdown() {
        let content = r#"---
capability: test-cap
stability: draft
guard:
  mode: protected
---
# Title
"#;
        let meta = LocsExtractor::extract_from_markdown(content).unwrap();
        assert_eq!(meta.capability.unwrap(), "test-cap");
        assert_eq!(meta.guard.unwrap().mode.unwrap(), GuardMode::Protected);
    }

    #[test]
    fn test_extract_comments() {
        let content = r#"
/*
LOCS:
  capability: comment-cap
  stability: active
  guard:
    mode: review_required
*/
"#;
        let meta = LocsExtractor::extract_from_comments(content).unwrap();
        assert_eq!(meta.capability.unwrap(), "comment-cap");
        assert_eq!(meta.guard.unwrap().mode.unwrap(), GuardMode::ReviewRequired);
    }

    #[test]
    fn test_find_metadata_line_range_frontmatter() {
        let content = "---\ncapability: x\n---\n# Title\n";
        let range = LocsExtractor::find_metadata_line_range(content);
        assert!(range.is_some(), "should find frontmatter range");
        let (start, end) = range.unwrap();
        assert_eq!(start, 0);
        assert!(end >= 2, "end should cover closing ---");
    }

    #[test]
    fn test_find_metadata_line_range_block_comment() {
        let content = "/*\nLOCS:\n  capability: y\n*/\npub fn f() {}\n";
        let range = LocsExtractor::find_metadata_line_range(content);
        assert!(range.is_some(), "should find LOCS block comment range");
        let (start, end) = range.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 3); // closing */ is on line 3 (0-indexed)
    }

    #[test]
    fn test_find_metadata_line_range_none_for_plain_file() {
        let content = "pub fn foo() {}\npub fn bar() {}\n";
        let range = LocsExtractor::find_metadata_line_range(content);
        assert!(range.is_none());
    }

    #[test]
    fn test_guard_config_region_fields_round_trip() {
        let yaml = r#"
mode: editable
locked_regions:
  - metadata
  - public-interface
editable_regions:
  - implementation
evidence_required:
  - tests-pass
"#;
        let guard: GuardConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(guard.locked_regions.as_ref().unwrap(), &["metadata", "public-interface"]);
        assert_eq!(guard.editable_regions.as_ref().unwrap(), &["implementation"]);
        assert_eq!(guard.evidence_required.as_ref().unwrap(), &["tests-pass"]);
    }
}
