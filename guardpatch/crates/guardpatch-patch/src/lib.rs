pub mod structured;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOperation {
    pub file: PathBuf,
    pub old_range: Range,
    pub new_range: Range,
    pub lines: Vec<PatchLine>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: usize,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchLine {
    Context(String),
    Add(String),
    Remove(String),
}

pub struct UnifiedDiffParser;

impl UnifiedDiffParser {
    pub fn parse(diff: &str) -> anyhow::Result<Vec<PatchOperation>> {
        let patches = patch::Patch::from_multiple(diff)
            .map_err(|e| anyhow::anyhow!("Failed to parse diff: {}", e))?;

        let mut operations = Vec::new();

        for p in patches {
            // Prefer new path if it exists (for additions/moves), otherwise old path
            let file_path = if p.new.path != "/dev/null" { &p.new.path } else { &p.old.path };
            let file = PathBuf::from(file_path.as_ref());
            
            for hunk in p.hunks {
                operations.push(PatchOperation {
                    file: file.clone(),
                    old_range: Range {
                        start: hunk.old_range.start as usize,
                        count: hunk.old_range.count as usize,
                    },
                    new_range: Range {
                        start: hunk.new_range.start as usize,
                        count: hunk.new_range.count as usize,
                    },
                    lines: hunk.lines.into_iter().map(|line| {
                        match line {
                            patch::Line::Context(s) => PatchLine::Context(s.to_string()),
                            patch::Line::Add(s) => PatchLine::Add(s.to_string()),
                            patch::Line::Remove(s) => PatchLine::Remove(s.to_string()),
                        }
                    }).collect(),
                });
            }
        }

        Ok(operations)
    }
}

pub struct PatchApplier;

impl PatchApplier {
    pub fn apply(lines: &[String], operations: &[PatchOperation]) -> anyhow::Result<Vec<String>> {
        let mut result = lines.to_vec();
        let mut offset: i64 = 0;

        for op in operations {
            let start = (op.old_range.start as i64 + offset - 1).max(0) as usize;
            let count = op.old_range.count;
            
            let mut new_lines = Vec::new();
            for line in &op.lines {
                match line {
                    PatchLine::Context(s) => new_lines.push(s.clone()),
                    PatchLine::Add(s) => new_lines.push(s.clone()),
                    PatchLine::Remove(_) => {}
                }
            }

            if start + count <= result.len() {
                result.splice(start..start + count, new_lines.clone());
            } else if start == result.len() && count == 0 {
                result.extend(new_lines.clone());
            } else {
                anyhow::bail!("Patch range out of bounds: start={}, count={}, len={}", start, count, result.len());
            }

            offset += new_lines.len() as i64 - count as i64;
        }

        Ok(result)
    }
}

pub use structured::{StructuredPatch, StructuredOperation};
