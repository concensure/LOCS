use guardpatch_policy::{GuardMode, SectionRole};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerRange {
    pub id: String,
    pub mode: GuardMode,
    pub role: Option<SectionRole>,
    pub start_line: usize, // 1-based
    pub end_line: usize,   // 1-based, inclusive
}

pub struct MarkerParser;

impl MarkerParser {
    pub fn parse(content: &str) -> anyhow::Result<Vec<MarkerRange>> {
        let mut markers = Vec::new();
        let mut stack: Vec<(String, GuardMode, Option<SectionRole>, usize)> = Vec::new();

        // Legacy GUARD: markers
        let re_start_legacy = Regex::new(r"GUARD:(LOCKED|EDITABLE|PROPOSAL_ONLY)\s+id=([^\s\- >]+)")?;
        let re_end_legacy = Regex::new(r"/GUARD:(LOCKED|EDITABLE|PROPOSAL_ONLY)")?;

        // New locs:section markers
        // locs:section id=... edit=... role=...
        let re_start_locs = Regex::new(r"locs:section\s+id=([^\s]+)(?:\s+edit=([^\s]+))?(?:\s+role=([^\s]+))?")?;
        let re_end_locs = Regex::new(r"locs:end")?;

        for (i, line) in content.lines().enumerate() {
            let line_num = i + 1;

            if let Some(caps) = re_start_legacy.captures(line) {
                let mode_str = caps.get(1).unwrap().as_str();
                let id = caps.get(2).unwrap().as_str().to_string();
                let mode = match mode_str {
                    "LOCKED" => GuardMode::Protected,
                    "EDITABLE" => GuardMode::Editable,
                    "PROPOSAL_ONLY" => GuardMode::ProposalOnly,
                    _ => unreachable!(),
                };
                stack.push((id, mode, None, line_num));
            } else if let Some(caps) = re_end_legacy.captures(line) {
                let mode_str = caps.get(1).unwrap().as_str();
                if let Some((id, mode, _, start_line)) = stack.pop() {
                    let expected_mode = match mode_str {
                        "LOCKED" => GuardMode::Protected,
                        "EDITABLE" => GuardMode::Editable,
                        "PROPOSAL_ONLY" => GuardMode::ProposalOnly,
                        _ => unreachable!(),
                    };
                    if mode != expected_mode {
                        anyhow::bail!("Mismatched guard markers at line {}: expected /GUARD:{:?}, found /GUARD:{}", line_num, mode, mode_str);
                    }
                    markers.push(MarkerRange {
                        id,
                        mode,
                        role: None,
                        start_line,
                        end_line: line_num,
                    });
                } else {
                    anyhow::bail!("Unbalanced end marker at line {}", line_num);
                }
            } else if let Some(caps) = re_start_locs.captures(line) {
                let id = caps.get(1).unwrap().as_str().to_string();
                let mode = caps.get(2).map(|m| match m.as_str() {
                    "locked" | "protected" => GuardMode::Protected,
                    "editable" => GuardMode::Editable,
                    "approval" | "review_required" => GuardMode::ReviewRequired,
                    _ => GuardMode::Protected, // Default to safe
                }).unwrap_or(GuardMode::Protected);
                
                let role = caps.get(3).and_then(|m| match m.as_str() {
                    "metadata" => Some(SectionRole::Metadata),
                    "contract" => Some(SectionRole::Contract),
                    "implementation" => Some(SectionRole::Implementation),
                    "example" => Some(SectionRole::Example),
                    "notes" => Some(SectionRole::Notes),
                    _ => None,
                });
                
                stack.push((id, mode, role, line_num));
            } else if re_end_locs.is_match(line) {
                if let Some((id, mode, role, start_line)) = stack.pop() {
                    markers.push(MarkerRange {
                        id,
                        mode,
                        role,
                        start_line,
                        end_line: line_num,
                    });
                } else {
                    anyhow::bail!("Unbalanced locs:end at line {}", line_num);
                }
            }
        }

        if !stack.is_empty() {
            let (id, mode, _, line) = stack.last().unwrap();
            anyhow::bail!("Unclosed guard marker {:?} ({:?}) starting at line {}", id, mode, line);
        }

        Ok(markers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_locs_section() {
        let content = r#"// locs:section id=impl edit=locked role=implementation
fn test() {}
// locs:end
"#;
        let markers = MarkerParser::parse(content).unwrap();
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].id, "impl");
        assert_eq!(markers[0].mode, GuardMode::Protected);
        assert_eq!(markers[0].role, Some(SectionRole::Implementation));
        assert_eq!(markers[0].start_line, 1);
        assert_eq!(markers[0].end_line, 3);
    }

    #[test]
    fn test_parse_legacy_guard() {
        let content = r#"<!-- GUARD:LOCKED id=header -->
<h1>Title</h1>
<!-- /GUARD:LOCKED -->
"#;
        let markers = MarkerParser::parse(content).unwrap();
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].id, "header");
        assert_eq!(markers[0].mode, GuardMode::Protected);
        assert_eq!(markers[0].start_line, 1);
        assert_eq!(markers[0].end_line, 3);
    }
}
