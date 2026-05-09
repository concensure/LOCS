use guardpatch_core::Decision;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationReport {
    pub timestamp: DateTime<Utc>,
    pub decision: Decision,
    pub files_checked: Vec<PathBuf>,
    pub summary: String,
    pub lines_changed: usize,
    pub protected_symbols_touched: usize,
}

impl VerificationReport {
    pub fn new(
        decision: Decision,
        files_checked: Vec<PathBuf>,
        lines_changed: usize,
        protected_symbols_touched: usize,
    ) -> Self {
        let summary = match &decision {
            Decision::Allowed => "Patch verified and allowed.".to_string(),
            Decision::Rejected(reason) => format!("Patch rejected: {}", reason),
            Decision::ReviewRequired(reason) => format!("Patch requires manual review: {}", reason),
            Decision::ProposalOnly(reason) => format!("Patch accepted as proposal only: {}", reason),
        };

        Self {
            timestamp: Utc::now(),
            decision,
            files_checked,
            summary,
            lines_changed,
            protected_symbols_touched,
        }
    }

    pub fn to_human_string(&self) -> String {
        let mut out = String::from("--- GuardPatch Report ---\n");
        out.push_str(&format!("Status:   {:?}\n", self.decision));
        out.push_str(&format!("Summary:  {}\n", self.summary));
        out.push_str(&format!("Files:    {:?}\n", self.files_checked));
        out.push_str(&format!("Lines:    {}\n", self.lines_changed));
        out.push_str("-------------------------\n");
        out
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Structured record written to `.guardpatch/ledger.jsonl` for every accepted edit.
/// Mirrors the shape described in the LOCS v2 evidence ledger spec.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeLedgerEntry {
    pub change_id: String,
    pub timestamp: DateTime<Utc>,
    /// Primary file (module) modified by the patch.
    pub module_id: String,
    /// Region names or file paths that were changed.
    pub changed_regions: Vec<String>,
    /// "allowed", "rejected", "review_required", or "proposal_only".
    pub policy_result: String,
    pub tests_passed: Option<bool>,
    pub typecheck_passed: Option<bool>,
    pub human_approval: bool,
    pub risk_score: u32,
    pub actor: String,
}

impl ChangeLedgerEntry {
    pub fn new(
        module_id: impl Into<String>,
        changed_regions: Vec<String>,
        decision: &Decision,
        risk_score: u32,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            change_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            module_id: module_id.into(),
            changed_regions,
            policy_result: match decision {
                Decision::Allowed => "allowed",
                Decision::Rejected(_) => "rejected",
                Decision::ReviewRequired(_) => "review_required",
                Decision::ProposalOnly(_) => "proposal_only",
            }.to_string(),
            tests_passed: None,
            typecheck_passed: None,
            human_approval: false,
            risk_score,
            actor: actor.into(),
        }
    }
}

pub struct EvidenceLedger {
    pub log_path: PathBuf,
}

impl EvidenceLedger {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { log_path: path.into() }
    }

    pub fn record(&self, entry: &ChangeLedgerEntry) -> anyhow::Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(&self.log_path)?;
        writeln!(file, "{}", serde_json::to_string(entry)?)?;
        Ok(())
    }

    pub fn load_recent(&self, limit: usize) -> anyhow::Result<Vec<ChangeLedgerEntry>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }
        let reader = BufReader::new(File::open(&self.log_path)?);
        let mut entries: Vec<ChangeLedgerEntry> = reader
            .lines()
            .filter_map(|l| l.ok())
            .filter_map(|l| serde_json::from_str(&l).ok())
            .collect();
        entries.reverse();
        entries.truncate(limit);
        Ok(entries)
    }
}

pub struct AuditStore {
    pub log_path: PathBuf,
}

impl AuditStore {
    pub fn new<P: Into<PathBuf>>(log_path: P) -> Self {
        Self { log_path: log_path.into() }
    }

    pub fn record_event(&self, report: &VerificationReport) -> anyhow::Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        let line = serde_json::to_string(report)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    pub fn load_recent(&self, limit: usize) -> anyhow::Result<Vec<VerificationReport>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut reports = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if let Ok(report) = serde_json::from_str::<VerificationReport>(&line) {
                reports.push(report);
            }
        }

        Ok(reports.into_iter().rev().take(limit).collect())
    }
}
