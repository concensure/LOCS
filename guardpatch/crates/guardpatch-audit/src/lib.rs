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
    /// Suggested remediation step for Rejected/ReviewRequired decisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_hint: Option<String>,
    /// Which policy rule triggered the decision (derived from the reason string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_source: Option<String>,
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

        let (fix_hint, rule_source) = Self::derive_hints(&decision, &files_checked);

        Self {
            timestamp: Utc::now(),
            decision,
            files_checked,
            summary,
            lines_changed,
            protected_symbols_touched,
            fix_hint,
            rule_source,
        }
    }

    fn derive_hints(decision: &Decision, files: &[PathBuf]) -> (Option<String>, Option<String>) {
        let file_hint = files.first()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<file>".to_string());

        match decision {
            Decision::Rejected(reason) => {
                let (hint, source) = if reason.contains("protected (mode=Protected)") || reason.contains("protected (mode=Frozen)") {
                    (
                        format!(
                            "Run: guardpatch unlock {} --reason \"<reason>\" --scope one_patch",
                            file_hint
                        ),
                        "path protection rule (.guardpatch.yml paths[])".to_string(),
                    )
                } else if reason.contains("locked symbol") {
                    (
                        "Remove the symbol from lock_symbols in .guardpatch.yml, or target a different symbol.".to_string(),
                        "lock_symbols (.guardpatch.yml)".to_string(),
                    )
                } else if reason.contains("locked signature") {
                    (
                        "Body edits are allowed; only the function signature is locked.".to_string(),
                        "lock_signatures (.guardpatch.yml)".to_string(),
                    )
                } else if reason.contains("locked section") {
                    (
                        "Use guardpatch unlock or remove the section from lock_sections in .guardpatch.yml.".to_string(),
                        "lock_sections (.guardpatch.yml)".to_string(),
                    )
                } else if reason.contains("locked first") {
                    (
                        "Edit lines past the locked header range, or reduce lock_first_lines in .guardpatch.yml.".to_string(),
                        "lock_first_lines (.guardpatch.yml)".to_string(),
                    )
                } else if reason.contains("Exported symbol") {
                    (
                        "Restore the exported symbol or disable lock_exports in .guardpatch.yml.".to_string(),
                        "lock_exports (.guardpatch.yml)".to_string(),
                    )
                } else if reason.contains("LOCS metadata") || reason.contains("Removal of LOCS") {
                    (
                        "Preserve the LOCS metadata block. Use guardpatch unlock if intentional metadata edits are needed.".to_string(),
                        "LOCS metadata protection (built-in)".to_string(),
                    )
                } else if reason.contains("Region") && reason.contains("locked by file policy") {
                    (
                        format!(
                            "Edit a different region, or remove the region from locked_regions in the LOCS guard block of {}.",
                            file_hint
                        ),
                        "per-file LOCS guard.locked_regions".to_string(),
                    )
                } else if reason.contains("Agent") && reason.contains("not permitted") {
                    (
                        "Check the agent's allow/deny patterns in .guardpatch.yml agents[].".to_string(),
                        "agent authority profile (.guardpatch.yml agents[])".to_string(),
                    )
                } else if reason.contains("exceeding limit") {
                    (
                        "Split the patch into smaller chunks, or raise patch_limits in .guardpatch.yml.".to_string(),
                        "patch_limits (.guardpatch.yml)".to_string(),
                    )
                } else {
                    (reason.clone(), "policy rule".to_string())
                };
                (Some(hint), Some(source))
            }
            Decision::ReviewRequired(reason) => {
                let hint = if reason.contains("evidence") {
                    format!(
                        "Run: guardpatch promote {} --to stable --evidence tests,typecheck",
                        file_hint
                    )
                } else if reason.contains("weakening") {
                    "Submit for human review: guardpatch review list".to_string()
                } else {
                    format!(
                        "Run: guardpatch review list  (or approve via: guardpatch review approve <id>)"
                    )
                };
                (Some(hint), Some("review_required policy".to_string()))
            }
            Decision::ProposalOnly(_) => (
                Some("Run: guardpatch review list  then: guardpatch review approve <id>".to_string()),
                Some("proposal_only agent mode".to_string()),
            ),
            Decision::Allowed => (None, None),
        }
    }

    pub fn to_human_string(&self) -> String {
        let mut out = String::from("--- GuardPatch Report ---\n");
        out.push_str(&format!("Status:   {:?}\n", self.decision));
        out.push_str(&format!("Summary:  {}\n", self.summary));
        if let Some(ref hint) = self.fix_hint {
            out.push_str(&format!("Fix:      {}\n", hint));
        }
        if let Some(ref source) = self.rule_source {
            out.push_str(&format!("Rule:     {}\n", source));
        }
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

    /// Count entries in the audit log without loading all into memory.
    pub fn entry_count(&self) -> anyhow::Result<usize> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        if !self.log_path.exists() {
            return Ok(0);
        }
        let reader = BufReader::new(File::open(&self.log_path)?);
        Ok(reader.lines().filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)).count())
    }

    /// Archive the current log to `audit.YYYY-MM-DD.jsonl` and start fresh.
    /// Only rotates if the current log has more than `max_entries` entries.
    /// Returns the archive path if rotation happened, or None if not needed.
    pub fn rotate(&self, max_entries: usize) -> anyhow::Result<Option<std::path::PathBuf>> {
        use std::fs;
        let count = self.entry_count()?;
        if count <= max_entries {
            return Ok(None);
        }

        let date_str = Utc::now().format("%Y-%m-%d").to_string();
        let archive_name = format!(
            "audit.{}.jsonl",
            date_str,
        );
        let archive_path = self.log_path.parent()
            .unwrap_or(std::path::Path::new("."))
            .join(&archive_name);

        // Resolve naming collisions with a counter suffix
        let archive_path = if archive_path.exists() {
            let mut i = 1u32;
            loop {
                let candidate = self.log_path.parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join(format!("audit.{}.{}.jsonl", date_str, i));
                if !candidate.exists() {
                    break candidate;
                }
                i += 1;
            }
        } else {
            archive_path
        };

        fs::rename(&self.log_path, &archive_path)?;
        Ok(Some(archive_path))
    }
}
