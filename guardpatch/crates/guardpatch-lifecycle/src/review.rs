use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Context;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewItem {
    pub id: String,
    pub patch_ref: String,
    pub reason: String,
    pub actor: String,
    pub status: ReviewStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_note: Option<String>,
}

impl ReviewItem {
    pub fn new(patch_ref: String, reason: String, actor: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            patch_ref,
            reason,
            actor,
            status: ReviewStatus::Pending,
            created_at: Utc::now(),
            resolved_at: None,
            resolution_note: None,
        }
    }
}

pub struct ReviewQueue {
    pub items: Vec<ReviewItem>,
    path: std::path::PathBuf,
}

impl ReviewQueue {
    pub fn load() -> anyhow::Result<Self> {
        std::fs::create_dir_all(".guardpatch")?;
        let path = std::path::PathBuf::from(".guardpatch/review_queue.jsonl");
        if !path.exists() {
            return Ok(Self { items: Vec::new(), path });
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| "Failed to read review_queue.jsonl")?;
        let items = content.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(Self { items, path })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let content = self.items.iter()
            .map(|item| serde_json::to_string(item))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        std::fs::write(&self.path, content + "\n")?;
        Ok(())
    }

    /// Add an item and return its ID.
    pub fn enqueue(&mut self, item: ReviewItem) -> String {
        let id = item.id.clone();
        self.items.push(item);
        id
    }

    pub fn pending(&self) -> Vec<&ReviewItem> {
        self.items.iter().filter(|i| i.status == ReviewStatus::Pending).collect()
    }

    pub fn approve(&mut self, id: &str) -> anyhow::Result<()> {
        let item = self.items.iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow::anyhow!("Review item not found: {}", id))?;
        item.status = ReviewStatus::Approved;
        item.resolved_at = Some(Utc::now());
        Ok(())
    }

    pub fn reject(&mut self, id: &str, reason: String) -> anyhow::Result<()> {
        let item = self.items.iter_mut()
            .find(|i| i.id == id)
            .ok_or_else(|| anyhow::anyhow!("Review item not found: {}", id))?;
        item.status = ReviewStatus::Rejected;
        item.resolved_at = Some(Utc::now());
        item.resolution_note = Some(reason);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_queue_lifecycle() {
        let mut rq = ReviewQueue { items: Vec::new(), path: ".guardpatch/review_queue.jsonl".into() };
        let item = ReviewItem::new("patch.diff".into(), "protected region touched".into(), "agent".into());
        let id = rq.enqueue(item);
        assert_eq!(rq.pending().len(), 1);
        rq.approve(&id).unwrap();
        assert_eq!(rq.pending().len(), 0);
        assert_eq!(rq.items[0].status, ReviewStatus::Approved);
    }

    #[test]
    fn test_reject_sets_note() {
        let mut rq = ReviewQueue { items: Vec::new(), path: ".guardpatch/review_queue.jsonl".into() };
        let item = ReviewItem::new("patch2.diff".into(), "reason".into(), "agent".into());
        let id = rq.enqueue(item);
        rq.reject(&id, "not approved by owner".into()).unwrap();
        assert_eq!(rq.items[0].resolution_note.as_deref(), Some("not approved by owner"));
    }
}
