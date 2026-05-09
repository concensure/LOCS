use guardpatch_policy::GuardMode;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerRange {
    pub id: String,
    pub mode: GuardMode,
    pub start_line: usize, // 1-based
    pub end_line: usize,   // 1-based, inclusive
}

pub struct MarkerParser;

impl MarkerParser {
    pub fn parse(content: &str) -> anyhow::Result<Vec<MarkerRange>> {
        let mut markers = Vec::new();
        let mut stack: Vec<(String, GuardMode, usize)> = Vec::new();

        let re_start = Regex::new(r"GUARD:(LOCKED|EDITABLE|PROPOSAL_ONLY)\s+id=([^\s->]+)")?;
        let re_end = Regex::new(r"/GUARD:(LOCKED|EDITABLE|PROPOSAL_ONLY)")?;

        for (i, line) in content.lines().enumerate() {
            let line_num = i + 1;

            if let Some(caps) = re_start.captures(line) {
                let mode_str = caps.get(1).unwrap().as_str();
                let id = caps.get(2).unwrap().as_str().to_string();
                let mode = match mode_str {
                    "LOCKED" => GuardMode::Protected,
                    "EDITABLE" => GuardMode::Editable,
                    "PROPOSAL_ONLY" => GuardMode::ProposalOnly,
                    _ => unreachable!(),
                };
                stack.push((id, mode, line_num));
            } else if let Some(caps) = re_end.captures(line) {
                let mode_str = caps.get(1).unwrap().as_str();
                if let Some((id, mode, start_line)) = stack.pop() {
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
                        start_line,
                        end_line: line_num,
                    });
                } else {
                    anyhow::bail!("Unbalanced end marker at line {}", line_num);
                }
            }
        }

        if !stack.is_empty() {
            let (id, mode, line) = stack.last().unwrap();
            anyhow::bail!("Unclosed guard marker {:?} ({:?}) starting at line {}", id, mode, line);
        }

        Ok(markers)
    }
}
