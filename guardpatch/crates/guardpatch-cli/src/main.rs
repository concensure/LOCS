use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use guardpatch_policy::Config;
use guardpatch_patch::{UnifiedDiffParser, PatchApplier, StructuredPatch};
use guardpatch_core::{Verifier, Decision, risk_score};
use guardpatch_audit::{VerificationReport, AuditStore, EvidenceLedger, ChangeLedgerEntry};
use guardpatch_lifecycle::{
    PromotionStore, PromotionState, UnlockRegistry, UnlockScope, EvidenceRunner,
    ReviewQueue, ReviewItem,
};

#[derive(Parser)]
#[command(name = "guardpatch")]
#[command(about = "Deterministic edit governance for LLM patches", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Actor performing the action (agent name or "human").
    #[arg(long, global = true)]
    actor: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialise a GuardPatch project (creates .guardpatch.yml and .guardpatch/ dir).
    Init,
    /// Scan the project and print protected surfaces.
    Scan,
    /// Print project status and active configuration.
    Status,
    /// Verify a patch file without applying it.
    Verify {
        patch: PathBuf,
        #[arg(long)]
        json: bool,
        /// Read a structured JSON patch from stdin instead of a unified diff file.
        #[arg(long)]
        stdin_json: bool,
    },
    /// Verify and apply a patch.
    Apply {
        patch: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Explain what a patch does and why it is allowed or rejected.
    Explain {
        patch: PathBuf,
    },
    /// Show the audit log.
    Audit {
        #[arg(long, default_value = "20")]
        limit: usize,
        #[arg(long)]
        report: bool,
    },
    /// Promote a file/symbol to a new stability state.
    Promote {
        target: String,
        #[arg(long)]
        to: String,
        /// Evidence flags: tests, typecheck, user_approval, release_tagged.
        #[arg(long, value_delimiter = ',')]
        evidence: Vec<String>,
    },
    /// Unlock a protected target for a limited scope.
    Unlock {
        target: String,
        #[arg(long)]
        reason: String,
        /// Scope: one_patch | branch | time_limited | review_required.
        #[arg(long, default_value = "one_patch")]
        scope: String,
        /// Duration in seconds for time_limited scope.
        #[arg(long)]
        expires_in: Option<u64>,
    },
    /// Relock a previously unlocked target.
    Relock {
        target: String,
    },
    /// Manage the review queue.
    Review {
        #[command(subcommand)]
        sub: ReviewCommands,
    },
    /// Show the evidence ledger (applied-patch records).
    Ledger {
        #[arg(long, default_value = "20")]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum ReviewCommands {
    /// List pending review items.
    List,
    /// Approve a patch in the review queue.
    Approve { id: String },
    /// Reject a patch in the review queue.
    Reject { id: String, #[arg(long)] reason: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::find_and_load()?.unwrap_or_else(|| {
        eprintln!("Warning: no .guardpatch.yml found. Run 'guardpatch init' to set up.");
        Config::default()
    });

    let audit_store = AuditStore::new(".guardpatch/audit.jsonl");
    let actor = cli.actor.as_deref().unwrap_or("unknown");

    match cli.command {
        Commands::Init => {
            fs::create_dir_all(".guardpatch")?;
            if !std::path::Path::new(".guardpatch.yml").exists() {
                fs::write(".guardpatch.yml", default_config_yaml())?;
                println!("Created .guardpatch.yml");
            } else {
                println!(".guardpatch.yml already exists.");
            }
            println!("Initialised GuardPatch project (actor: {}).", actor);
        }

        Commands::Scan => {
            println!("Scanning project for protected surfaces...");
            for rule in &config.paths {
                println!("  [{}] {}", format!("{:?}", rule.mode).to_uppercase(), rule.pattern);
            }
            for section in &config.lock_sections {
                println!("  [SECTION-LOCKED] {}", section);
            }
            for sym in &config.lock_symbols {
                println!("  [SYMBOL-LOCKED] {}", sym);
            }
        }

        Commands::Status => {
            let registry = PromotionStore::load()?;
            let unlocks = UnlockRegistry::load()?;
            println!("Project: {}", config.project.name);
            println!("Default mode: {:?}", config.project.mode);
            println!("Promoted targets: {}", registry.entries.len());
            println!("Active unlocks: {}", unlocks.active_count());
        }

        Commands::Verify { patch, json, stdin_json } => {
            let report = if stdin_json {
                verify_structured_stdin(&config)?
            } else {
                verify_diff(&config, &patch, actor)?
            };
            audit_store.record_event(&report)?;

            if json {
                println!("{}", report.to_json()?);
            } else {
                println!("{}", report.to_human_string());
            }

            if !matches!(report.decision, Decision::Allowed) {
                std::process::exit(1);
            }
        }

        Commands::Apply { patch, force } => {
            let report = verify_diff(&config, &patch, actor)?;
            audit_store.record_event(&report)?;

            match &report.decision {
                Decision::Allowed => {
                    apply_diff(&patch)?;
                    // Auto-consume one_patch unlocks
                    let mut unlocks = UnlockRegistry::load()?;
                    unlocks.consume_one_patch_unlocks();
                    unlocks.save()?;
                    // Record evidence ledger entry
                    let score = risk_score::compute_score(
                        &report.files_checked,
                        report.lines_changed,
                        report.protected_symbols_touched,
                    );
                    let module_id = report.files_checked.first()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| patch.display().to_string());
                    let changed_regions: Vec<String> = report.files_checked.iter()
                        .map(|p| p.display().to_string())
                        .collect();
                    let entry = ChangeLedgerEntry::new(module_id, changed_regions, &report.decision, score, actor);
                    EvidenceLedger::new(".guardpatch/ledger.jsonl").record(&entry)?;
                    println!("Patch applied successfully.");
                }
                _ if force => {
                    println!("WARNING: Forcing application of rejected patch!");
                    apply_diff(&patch)?;
                    println!("Patch applied with --force.");
                }
                Decision::Rejected(reason) => {
                    println!("ERROR: Patch rejected: {}", reason);
                    std::process::exit(1);
                }
                Decision::ReviewRequired(reason) => {
                    // Add to review queue
                    let mut rq = ReviewQueue::load()?;
                    let id = rq.enqueue(ReviewItem::new(
                        patch.display().to_string(),
                        reason.clone(),
                        actor.to_string(),
                    ));
                    rq.save()?;
                    println!("Patch queued for review (id: {}): {}", id, reason);
                    std::process::exit(2);
                }
                Decision::ProposalOnly(reason) => {
                    println!("Patch accepted as proposal only: {}", reason);
                    println!("Use 'guardpatch review approve' after human review.");
                    std::process::exit(2);
                }
            }
        }

        Commands::Explain { patch } => {
            let report = verify_diff(&config, &patch, actor)?;
            let score = risk_score::compute_score(&report.files_checked, report.lines_changed, report.protected_symbols_touched);
            println!("{}", report.to_human_string());
            println!("Risk score: {}/100", score);
        }

        Commands::Audit { limit, report } => {
            let recent = audit_store.load_recent(limit)?;
            if report {
                println!("=== GuardPatch Audit Report ===");
                println!("{:<20} {:<15} {}", "Timestamp", "Decision", "Summary");
                println!("{}", "-".repeat(80));
                for r in &recent {
                    println!(
                        "{:<20} {:<15} {}",
                        r.timestamp.format("%Y-%m-%dT%H:%M:%S"),
                        format!("{:?}", r.decision).chars().take(15).collect::<String>(),
                        r.summary
                    );
                }
                println!("{}", "-".repeat(80));
                let rejected = recent.iter().filter(|r| matches!(r.decision, Decision::Rejected(_))).count();
                let allowed = recent.iter().filter(|r| matches!(r.decision, Decision::Allowed)).count();
                println!("Allowed: {}  Rejected: {}  Total: {}", allowed, rejected, recent.len());
            } else {
                for r in recent {
                    println!("[{}] {:?} - {}", r.timestamp.format("%Y-%m-%dT%H:%M:%S"), r.decision, r.summary);
                }
            }
        }

        Commands::Promote { target, to, evidence } => {
            let state = PromotionState::from_str(&to)
                .ok_or_else(|| anyhow::anyhow!("Unknown promotion state: {}. Valid: draft, active, stabilising, stable, protected, frozen", to))?;

            // Run required evidence checks
            let runner = EvidenceRunner::new(&config);
            for ev in &evidence {
                match ev.as_str() {
                    "tests" => {
                        let result = runner.run_tests()?;
                        if !result.passed {
                            anyhow::bail!("Evidence check failed: tests did not pass\n{}", result.output);
                        }
                        println!("  [OK] tests passed");
                    }
                    "typecheck" => {
                        let result = runner.run_typecheck()?;
                        if !result.passed {
                            anyhow::bail!("Evidence check failed: typecheck did not pass\n{}", result.output);
                        }
                        println!("  [OK] typecheck passed");
                    }
                    "user_approval" => println!("  [OK] user_approval recorded"),
                    "release_tagged" => println!("  [OK] release_tagged recorded"),
                    other => println!("  [?] unknown evidence kind: {}", other),
                }
            }

            let mut store = PromotionStore::load()?;
            store.promote(&target, state.clone(), actor.to_string(), evidence)?;
            store.save()?;
            println!("Promoted '{}' to {:?}.", target, state);
        }

        Commands::Unlock { target, reason, scope, expires_in } => {
            let scope = UnlockScope::from_str(&scope)
                .ok_or_else(|| anyhow::anyhow!("Unknown scope: {}. Valid: one_patch, branch, time_limited, review_required", scope))?;

            let mut registry = UnlockRegistry::load()?;
            let id = registry.add_unlock(&target, reason.clone(), scope, expires_in, actor.to_string());
            registry.save()?;
            let scope_str = registry.get(id).map(|u| u.scope.to_str()).unwrap_or("?");
            println!("Unlocked '{}' (id: {}, scope: {}, reason: {}).", target, id, scope_str, reason);
        }

        Commands::Relock { target } => {
            let mut registry = UnlockRegistry::load()?;
            let count = registry.relock(&target);
            registry.save()?;
            println!("Relocked '{}' ({} unlock(s) removed).", target, count);
        }

        Commands::Ledger { limit, json } => {
            let ledger = EvidenceLedger::new(".guardpatch/ledger.jsonl");
            let entries = ledger.load_recent(limit)?;
            if entries.is_empty() {
                println!("No ledger entries yet. Apply a patch to create the first entry.");
            } else if json {
                println!("{}", serde_json::to_string_pretty(&entries)?);
            } else {
                println!("{:<38} {:<20} {:<15} {}", "change_id", "module", "result", "risk");
                println!("{}", "-".repeat(90));
                for e in &entries {
                    let short_id = &e.change_id[..8];
                    let module = e.module_id.chars().take(18).collect::<String>();
                    println!("{:<38} {:<20} {:<15} {}", short_id, module, e.policy_result, e.risk_score);
                }
            }
        }

        Commands::Review { sub } => {
            let mut rq = ReviewQueue::load()?;
            match sub {
                ReviewCommands::List => {
                    let pending = rq.pending();
                    if pending.is_empty() {
                        println!("No pending review items.");
                    } else {
                        for item in pending {
                            println!("[{}] {} — {} (by {})", item.id, item.patch_ref, item.reason, item.actor);
                        }
                    }
                }
                ReviewCommands::Approve { id } => {
                    rq.approve(&id)?;
                    rq.save()?;
                    println!("Approved review item {}.", id);
                }
                ReviewCommands::Reject { id, reason } => {
                    rq.reject(&id, reason)?;
                    rq.save()?;
                    println!("Rejected review item {}.", id);
                }
            }
        }
    }

    Ok(())
}

fn verify_diff(config: &Config, patch_path: &PathBuf, actor: &str) -> anyhow::Result<VerificationReport> {
    let diff = fs::read_to_string(patch_path)?;
    let operations = UnifiedDiffParser::parse(&diff)?;

    let unlocks = UnlockRegistry::load()?;
    let active_targets: Vec<String> = unlocks.active_targets();

    let mut files_checked = Vec::new();
    let mut decision = Decision::Allowed;
    let mut lines_changed: usize = 0;
    let mut protected_symbols_touched: usize = 0;

    for op in &operations {
        if !files_checked.contains(&op.file) {
            files_checked.push(op.file.clone());
        }
        lines_changed += op.old_range.count + op.new_range.count;

        let content = if op.file.exists() {
            Some(fs::read_to_string(&op.file)?)
        } else {
            None
        };

        let op_decision = Verifier::verify_patch(
            config,
            &[op.clone()],
            content.as_deref(),
            None,
            Some(actor),
            &active_targets,
        );

        if let Decision::Rejected(_) = &op_decision {
            protected_symbols_touched += 1;
        }

        match op_decision {
            Decision::Allowed => {}
            _ => {
                decision = op_decision;
                break;
            }
        }
    }

    Ok(VerificationReport::new(decision, files_checked, lines_changed, protected_symbols_touched))
}

fn verify_structured_stdin(config: &Config) -> anyhow::Result<VerificationReport> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let patch = StructuredPatch::from_json(&buf)?;

    let unlocks = UnlockRegistry::load()?;
    let active_targets = unlocks.active_targets();

    let decision = Verifier::verify_structured_patch(config, &patch, None, &active_targets);
    Ok(VerificationReport::new(decision, vec![], 0, 0))
}

fn apply_diff(patch_path: &PathBuf) -> anyhow::Result<()> {
    let diff = fs::read_to_string(patch_path)?;
    let operations = UnifiedDiffParser::parse(&diff)?;

    use std::collections::HashMap;
    let mut file_ops: HashMap<PathBuf, Vec<_>> = HashMap::new();
    for op in operations {
        file_ops.entry(op.file.clone()).or_default().push(op);
    }

    for (file, ops) in file_ops {
        let lines = if file.exists() {
            fs::read_to_string(&file)?.lines().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        let new_lines = PatchApplier::apply(&lines, &ops)?;
        fs::write(&file, new_lines.join("\n") + "\n")?;
    }

    Ok(())
}

fn default_config_yaml() -> &'static str {
    r#"version: 1

project:
  name: my-project
  mode: editable
  locs_required_for_new_files: false

paths: []

lock_sections: []
lock_symbols: []
lock_exports: false
lock_dependencies: false
detect_test_weakening: true

patch_limits:
  max_files_changed: 10
  max_lines_changed: 500
  dependency_changes_require_approval: true

unlock_policy:
  require_reason: true
  auto_relock_after_merge: true
"#
}
