use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use anyhow::Context;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnlockScope {
    /// Consumed after the next verified patch.
    OnePatch,
    /// Valid for the current git branch.
    Branch,
    /// Valid until `expires_at`.
    TimeLimited,
    /// Patch is allowed but still requires review-queue approval.
    ReviewRequired,
}

impl UnlockScope {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "one_patch" => Some(Self::OnePatch),
            "branch" => Some(Self::Branch),
            "time_limited" => Some(Self::TimeLimited),
            "review_required" => Some(Self::ReviewRequired),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::OnePatch => "one_patch",
            Self::Branch => "branch",
            Self::TimeLimited => "time_limited",
            Self::ReviewRequired => "review_required",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockEntry {
    pub id: u64,
    pub target: String,
    pub reason: String,
    pub scope: UnlockScope,
    pub actor: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    /// For OnePatch scope: whether this unlock has been used.
    pub consumed: bool,
}

pub struct UnlockRegistry {
    pub entries: Vec<UnlockEntry>,
    next_id: u64,
    path: std::path::PathBuf,
}

impl UnlockRegistry {
    pub fn load() -> anyhow::Result<Self> {
        std::fs::create_dir_all(".guardpatch")?;
        let path = std::path::PathBuf::from(".guardpatch/unlocks.json");
        if !path.exists() {
            return Ok(Self { entries: Vec::new(), next_id: 1, path });
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| "Failed to read unlocks.json")?;

        #[derive(Deserialize)]
        struct Store {
            entries: Vec<UnlockEntry>,
            next_id: u64,
        }
        let store: Store = serde_json::from_str(&content)
            .with_context(|| "Failed to parse unlocks.json")?;
        Ok(Self { entries: store.entries, next_id: store.next_id, path })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        #[derive(Serialize)]
        struct Store<'a> {
            entries: &'a Vec<UnlockEntry>,
            next_id: u64,
        }
        let content = serde_json::to_string_pretty(&Store { entries: &self.entries, next_id: self.next_id })?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    /// Add a new unlock. Returns the assigned ID.
    pub fn add_unlock(
        &mut self,
        target: &str,
        reason: String,
        scope: UnlockScope,
        expires_in_seconds: Option<u64>,
        actor: String,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let expires_at = expires_in_seconds.map(|secs| Utc::now() + Duration::seconds(secs as i64));
        self.entries.push(UnlockEntry {
            id,
            target: target.to_string(),
            reason,
            scope,
            actor,
            created_at: Utc::now(),
            expires_at,
            consumed: false,
        });
        id
    }

    pub fn get(&self, id: u64) -> Option<&UnlockEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Returns targets that currently have an active unlock.
    pub fn active_targets(&self) -> Vec<String> {
        let now = Utc::now();
        self.entries.iter()
            .filter(|e| {
                !e.consumed
                    && e.scope != UnlockScope::ReviewRequired
                    && e.expires_at.map(|exp| exp > now).unwrap_or(true)
            })
            .map(|e| e.target.clone())
            .collect()
    }

    pub fn active_count(&self) -> usize {
        self.active_targets().len()
    }

    /// Mark all OnePatch unlocks as consumed (called after a patch is applied).
    pub fn consume_one_patch_unlocks(&mut self) {
        for entry in &mut self.entries {
            if entry.scope == UnlockScope::OnePatch && !entry.consumed {
                entry.consumed = true;
            }
        }
    }

    /// Remove all unlocks for `target`. Returns how many were removed.
    pub fn relock(&mut self, target: &str) -> usize {
        let before = self.entries.len();
        self.entries.retain(|e| e.target != target);
        before - self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_round_trip() {
        for s in ["one_patch", "branch", "time_limited", "review_required"] {
            let scope = UnlockScope::from_str(s).unwrap();
            assert_eq!(scope.to_str(), s);
        }
    }

    #[test]
    fn test_active_targets_excludes_consumed() {
        let mut reg = UnlockRegistry { entries: Vec::new(), next_id: 1, path: ".guardpatch/unlocks.json".into() };
        reg.add_unlock("src/foo.ts", "testing".into(), UnlockScope::OnePatch, None, "user".into());
        assert_eq!(reg.active_targets().len(), 1);
        reg.consume_one_patch_unlocks();
        assert_eq!(reg.active_targets().len(), 0);
    }

    #[test]
    fn test_relock_removes_entries() {
        let mut reg = UnlockRegistry { entries: Vec::new(), next_id: 1, path: ".guardpatch/unlocks.json".into() };
        reg.add_unlock("src/foo.ts", "r1".into(), UnlockScope::Branch, None, "user".into());
        reg.add_unlock("src/bar.ts", "r2".into(), UnlockScope::Branch, None, "user".into());
        let removed = reg.relock("src/foo.ts");
        assert_eq!(removed, 1);
        assert_eq!(reg.entries.len(), 1);
    }
}
