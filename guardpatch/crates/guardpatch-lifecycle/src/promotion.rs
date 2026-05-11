use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Context;

/// Linear promotion states for a file or symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionState {
    Draft,
    Active,
    Stabilising,
    Stable,
    Protected,
    Frozen,
}

impl PromotionState {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "active" => Some(Self::Active),
            "stabilising" => Some(Self::Stabilising),
            "stable" => Some(Self::Stable),
            "protected" => Some(Self::Protected),
            "frozen" => Some(Self::Frozen),
            _ => None,
        }
    }

    pub fn priority(&self) -> u32 {
        match self {
            Self::Draft => 0,
            Self::Active => 1,
            Self::Stabilising => 2,
            Self::Stable => 3,
            Self::Protected => 4,
            Self::Frozen => 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionRecord {
    pub target: String,
    pub state: PromotionState,
    pub history: Vec<PromotionEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionEvent {
    pub from: Option<PromotionState>,
    pub to: PromotionState,
    pub actor: String,
    pub evidence: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct PromotionStore {
    pub entries: HashMap<String, PromotionRecord>,
    path: std::path::PathBuf,
}

impl PromotionStore {
    pub fn load() -> anyhow::Result<Self> {
        std::fs::create_dir_all(".guardpatch")?;
        let path = std::path::PathBuf::from(".guardpatch/promotion.json");
        if !path.exists() {
            return Ok(Self { entries: HashMap::new(), path });
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| "Failed to read promotion.json")?;
        let entries: HashMap<String, PromotionRecord> = serde_json::from_str(&content)
            .with_context(|| "Failed to parse promotion.json")?;
        Ok(Self { entries, path })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    pub fn promote(
        &mut self,
        target: &str,
        to: PromotionState,
        actor: String,
        evidence: Vec<String>,
    ) -> anyhow::Result<()> {
        // Enforce mandatory evidence floor for high-stability states.
        // Protected and Frozen cannot be reached without at least one evidence item.
        if matches!(to, PromotionState::Protected | PromotionState::Frozen) && evidence.is_empty() {
            anyhow::bail!(
                "Promoting '{}' to {:?} requires at least one evidence item \
                 (e.g. --evidence tests,typecheck or --evidence user_approval). \
                 These stability levels enforce an immutable audit trail.",
                target, to
            );
        }

        let entry = self.entries.entry(target.to_string()).or_insert_with(|| PromotionRecord {
            target: target.to_string(),
            state: PromotionState::Draft,
            history: Vec::new(),
        });

        let from = Some(entry.state.clone());
        entry.history.push(PromotionEvent {
            from,
            to: to.clone(),
            actor,
            evidence,
            timestamp: chrono::Utc::now(),
        });
        entry.state = to;
        Ok(())
    }

    pub fn get_state(&self, target: &str) -> Option<&PromotionState> {
        self.entries.get(target).map(|e| &e.state)
    }
}

/// Check stable commit count for a file using git CLI.
pub struct GitHistoryChecker;

impl GitHistoryChecker {
    /// Count commits touching this file in the last N commits.
    pub fn count_file_commits(file: &str) -> anyhow::Result<usize> {
        let output = std::process::Command::new("git")
            .args(["log", "--oneline", "--follow", "--", file])
            .output()
            .with_context(|| "Failed to run git log")?;

        if !output.status.success() {
            anyhow::bail!("git log failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let count = String::from_utf8_lossy(&output.stdout).lines().count();
        Ok(count)
    }

    /// Count commits that did NOT touch this file among the last N total commits.
    /// Returns the number of stable (no-touch) commits in a window.
    pub fn stable_commit_count(file: &str, window: usize) -> anyhow::Result<usize> {
        let total_output = std::process::Command::new("git")
            .args(["log", "--oneline", "-n", &window.to_string()])
            .output()
            .with_context(|| "Failed to run git log")?;

        let total = String::from_utf8_lossy(&total_output.stdout).lines().count();

        let touching_output = std::process::Command::new("git")
            .args(["log", "--oneline", "-n", &window.to_string(), "--", file])
            .output()
            .with_context(|| "Failed to run git log for file")?;

        let touching = String::from_utf8_lossy(&touching_output.stdout).lines().count();
        Ok(total.saturating_sub(touching))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_promotion_state_from_str() {
        assert_eq!(PromotionState::from_str("draft"), Some(PromotionState::Draft));
        assert_eq!(PromotionState::from_str("frozen"), Some(PromotionState::Frozen));
        assert_eq!(PromotionState::from_str("unknown"), None);
    }

    #[test]
    fn test_promotion_state_priority_order() {
        assert!(PromotionState::Draft.priority() < PromotionState::Active.priority());
        assert!(PromotionState::Stable.priority() < PromotionState::Protected.priority());
        assert!(PromotionState::Protected.priority() < PromotionState::Frozen.priority());
    }

    #[test]
    fn test_promote_to_protected_requires_evidence() {
        let mut store = PromotionStore { entries: std::collections::HashMap::new(), path: std::path::PathBuf::from(".guardpatch/promotion.json") };
        let result = store.promote("src/core.ts", PromotionState::Protected, "human".to_string(), vec![]);
        assert!(result.is_err(), "promoting to Protected without evidence should fail");
        assert!(result.unwrap_err().to_string().contains("evidence"));
    }

    #[test]
    fn test_promote_to_frozen_requires_evidence() {
        let mut store = PromotionStore { entries: std::collections::HashMap::new(), path: std::path::PathBuf::from(".guardpatch/promotion.json") };
        let result = store.promote("src/core.ts", PromotionState::Frozen, "human".to_string(), vec![]);
        assert!(result.is_err(), "promoting to Frozen without evidence should fail");
    }

    #[test]
    fn test_promote_to_protected_with_evidence_succeeds() {
        let mut store = PromotionStore { entries: std::collections::HashMap::new(), path: std::path::PathBuf::from(".guardpatch/promotion.json") };
        let result = store.promote("src/core.ts", PromotionState::Protected, "human".to_string(), vec!["tests".to_string(), "user_approval".to_string()]);
        assert!(result.is_ok(), "promoting to Protected with evidence should succeed");
    }

    #[test]
    fn test_promote_to_stable_without_evidence_allowed() {
        let mut store = PromotionStore { entries: std::collections::HashMap::new(), path: std::path::PathBuf::from(".guardpatch/promotion.json") };
        let result = store.promote("src/core.ts", PromotionState::Stable, "human".to_string(), vec![]);
        assert!(result.is_ok(), "promoting to Stable without evidence should still be allowed");
    }
}
