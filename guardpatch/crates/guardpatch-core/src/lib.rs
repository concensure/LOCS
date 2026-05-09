pub mod symbol_registry;
pub mod risk_score;

use guardpatch_policy::{Config, GuardMode};
use guardpatch_patch::PatchOperation;
use guardpatch_parse::{MarkerParser, MarkdownParser, ParserRegistry, InlineMarkdownPolicy};
use guardpatch_locs::LocsExtractor;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub use symbol_registry::SymbolRegistry;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Decision {
    Allowed,
    Rejected(String),
    ReviewRequired(String),
    ProposalOnly(String),
}

pub struct Verifier;

impl Verifier {
    /// Verify a set of patch operations against policy.
    ///
    /// - `actor`: optional name of the acting agent (for per-agent authority checks)
    /// - `active_unlocks`: paths/targets that have been explicitly unlocked
    pub fn verify_patch(
        config: &Config,
        operations: &[PatchOperation],
        file_content: Option<&str>,
        symbol_registry: Option<&SymbolRegistry>,
        actor: Option<&str>,
        active_unlocks: &[String],
    ) -> Decision {
        if operations.is_empty() {
            return Decision::Allowed;
        }

        let markers = if let Some(content) = file_content {
            MarkerParser::parse(content).unwrap_or_default()
        } else {
            Vec::new()
        };

        let sections = if let Some(content) = file_content {
            if let Some(ext) = Path::new(&operations[0].file).extension() {
                if ext == "md" || ext == "markdown" {
                    MarkdownParser::parse_sections(content)
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Per-document inline policy from <!-- guardpatch-locked: ... --> HTML comments
        let inline_policy = if let Some(content) = file_content {
            if let Some(ext) = Path::new(&operations[0].file).extension() {
                if ext == "md" || ext == "markdown" {
                    MarkdownParser::parse_inline_policy(content)
                } else {
                    InlineMarkdownPolicy::default()
                }
            } else {
                InlineMarkdownPolicy::default()
            }
        } else {
            InlineMarkdownPolicy::default()
        };

        let locs_meta = if let Some(content) = file_content {
            LocsExtractor::extract_from_markdown(content)
                .or_else(|| LocsExtractor::extract_from_comments(content))
        } else {
            None
        };

        // Check patch_limits
        if let Some(max_files) = config.patch_limits.max_files_changed {
            let unique_files: std::collections::HashSet<_> = operations.iter().map(|o| &o.file).collect();
            if unique_files.len() > max_files {
                return Decision::Rejected(format!(
                    "Patch changes {} files, exceeding limit of {}",
                    unique_files.len(), max_files
                ));
            }
        }

        if let Some(max_lines) = config.patch_limits.max_lines_changed {
            let total_lines: usize = operations.iter().map(|o| o.old_range.count + o.new_range.count).sum();
            if total_lines > max_lines {
                return Decision::Rejected(format!(
                    "Patch changes {} lines, exceeding limit of {}",
                    total_lines, max_lines
                ));
            }
        }

        // For weakening and export locking, we need candidate state
        if let Some(content) = file_content {
            use guardpatch_patch::PatchApplier;
            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            if let Ok(new_lines) = PatchApplier::apply(&lines, operations) {
                let new_content = new_lines.join("\n");

                // LOCS weakening check
                let new_locs_meta = LocsExtractor::extract_from_markdown(&new_content)
                    .or_else(|| LocsExtractor::extract_from_comments(&new_content));

                if let (Some(old), Some(new)) = (&locs_meta, &new_locs_meta) {
                    if Self::is_weakening(old, new) {
                        return Decision::ReviewRequired("LOCS governance weakening detected".to_string());
                    }
                } else if locs_meta.is_some() && new_locs_meta.is_none() {
                    return Decision::Rejected("Removal of LOCS metadata is not allowed".to_string());
                }

                // LOCS per-file region policy enforcement (6.1/6.2)
                if let Some(ref meta) = locs_meta {
                    if let Some(ref guard) = meta.guard {
                        // evidence_required: any non-empty list → ReviewRequired
                        if let Some(ref ev_req) = guard.evidence_required {
                            if !ev_req.is_empty() {
                                return Decision::ReviewRequired(format!(
                                    "File requires evidence before edit: {}",
                                    ev_req.join(", ")
                                ));
                            }
                        }

                        // locked_regions: reject if any changed region is in the locked list
                        if let Some(ref locked) = guard.locked_regions {
                            if !locked.is_empty() {
                                let locs_range = LocsExtractor::find_metadata_line_range(content);
                                for op in operations {
                                    let touched = Self::classify_regions_for_op(op, Some(content), locs_range);
                                    for region in &touched {
                                        if locked.iter().any(|r| r == region) {
                                            return Decision::Rejected(format!(
                                                "Region {:?} is locked by file policy in {:?}",
                                                region, op.file
                                            ));
                                        }
                                    }
                                }
                            }
                        }

                        // editable_regions: if all touched regions are explicitly editable, allow early
                        if let Some(ref editable) = guard.editable_regions {
                            if !editable.is_empty() {
                                let locs_range = LocsExtractor::find_metadata_line_range(content);
                                let all_editable = operations.iter().all(|op| {
                                    let touched = Self::classify_regions_for_op(op, Some(content), locs_range);
                                    !touched.is_empty()
                                        && touched.iter().all(|r| editable.iter().any(|e| e == r))
                                });
                                if all_editable {
                                    return Decision::Allowed;
                                }
                            }
                        }
                    }
                }

                // Exported API locking (3.7)
                if config.lock_exports {
                    if let Some(adapter) = ParserRegistry::get_adapter(Path::new(&operations[0].file)) {
                        let old_symbols = adapter.parse_symbols(content).unwrap_or_default();
                        let new_symbols = adapter.parse_symbols(&new_content).unwrap_or_default();

                        for old_s in old_symbols.iter().filter(|s| s.is_exported) {
                            let still_exists = new_symbols.iter().any(|s| s.name == old_s.name && s.is_exported);
                            if !still_exists {
                                return Decision::Rejected(format!(
                                    "Exported symbol {:?} was removed or unexported in {:?}",
                                    old_s.name, operations[0].file
                                ));
                            }
                        }
                    }
                }

                // Import/dependency drift checks (3.8)
                if config.lock_dependencies {
                    if let Some(adapter) = ParserRegistry::get_adapter(Path::new(&operations[0].file)) {
                        let old_symbols = adapter.parse_symbols(content).unwrap_or_default();
                        let new_symbols = adapter.parse_symbols(&new_content).unwrap_or_default();

                        let old_imports: Vec<_> = old_symbols.iter()
                            .filter(|s| matches!(s.kind, guardpatch_parse::SymbolKind::Import))
                            .collect();
                        let new_imports: Vec<_> = new_symbols.iter()
                            .filter(|s| matches!(s.kind, guardpatch_parse::SymbolKind::Import))
                            .collect();

                        for ni in &new_imports {
                            if !old_imports.iter().any(|oi| oi.name == ni.name) {
                                return Decision::ReviewRequired(format!(
                                    "New import detected: {:?} in {:?}",
                                    ni.name, operations[0].file
                                ));
                            }
                        }
                    }

                    let dep_files = ["Cargo.toml", "package.json", "go.mod", "requirements.txt", "pyproject.toml"];
                    let file_name = Path::new(&operations[0].file)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if dep_files.contains(&file_name) {
                        if config.patch_limits.dependency_changes_require_approval {
                            return Decision::ReviewRequired(format!(
                                "Direct modification of dependency file {:?} requires review",
                                file_name
                            ));
                        }
                    }
                }

                // Test weakening heuristics (3.9)
                if config.detect_test_weakening {
                    let file_path = Path::new(&operations[0].file);
                    let is_test_file = file_path.to_str()
                        .map(|s| s.contains("test") || s.contains("spec"))
                        .unwrap_or(false);

                    if is_test_file {
                        for op in operations {
                            for line in &op.lines {
                                if let guardpatch_patch::PatchLine::Remove(s) = line {
                                    if s.contains("assert") || s.contains("expect") {
                                        return Decision::ReviewRequired(format!(
                                            "Potential test weakening: removed assertion in {:?}", op.file
                                        ));
                                    }
                                }
                                if let guardpatch_patch::PatchLine::Add(s) = line {
                                    if s.contains("ignore") || s.contains("skip") || s.contains("todo!") {
                                        return Decision::ReviewRequired(format!(
                                            "Potential test weakening: added skip/ignore in {:?}", op.file
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for op in operations {
            let file_path_str = op.file.to_str().unwrap_or("");

            // Agent-aware authority check (5.2 / 5.3)
            if let Some(agent_name) = actor {
                if !config.agents.is_empty() {
                    if let Some(agent_mode) = config.resolve_agent_mode(agent_name, &op.file) {
                        match agent_mode {
                            GuardMode::Protected | GuardMode::Frozen | GuardMode::HumanOnly => {
                                // Check if this target is unlocked
                                if !Self::is_unlocked(file_path_str, active_unlocks) {
                                    return Decision::Rejected(format!(
                                        "Agent {:?} is not permitted to modify {:?}",
                                        agent_name, op.file
                                    ));
                                }
                            }
                            GuardMode::ProposalOnly => {
                                return Decision::ProposalOnly(format!(
                                    "Agent {:?} may only propose changes to {:?}",
                                    agent_name, op.file
                                ));
                            }
                            GuardMode::ReviewRequired => {
                                return Decision::ReviewRequired(format!(
                                    "Agent {:?} changes to {:?} require review",
                                    agent_name, op.file
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }

            let mut mode = config.resolve_path_mode(&op.file);

            // New file LOCS requirement check (2.6)
            let is_new_file = !op.file.exists() && op.old_range.start == 0 && op.old_range.count == 0;
            if is_new_file && config.project.locs_required_for_new_files {
                use guardpatch_patch::PatchApplier;
                if let Ok(new_lines) = PatchApplier::apply(&[], operations) {
                    let new_content = new_lines.join("\n");
                    let has_locs = LocsExtractor::extract_from_markdown(&new_content).is_some()
                        || LocsExtractor::extract_from_comments(&new_content).is_some();
                    if !has_locs {
                        return Decision::Rejected(format!("New file requires LOCS metadata: {:?}", op.file));
                    }
                }
            }

            // LOCS metadata overrides path mode if present
            if let Some(ref meta) = locs_meta {
                if let Some(ref guard) = meta.guard {
                    if let Some(ref guard_mode) = guard.mode {
                        mode = guard_mode.clone();
                    }
                }
            }

            // Unlock check: if target is unlocked, downgrade Protected/Frozen to ReviewRequired
            if Self::is_unlocked(file_path_str, active_unlocks) {
                mode = match mode {
                    GuardMode::Protected | GuardMode::Frozen | GuardMode::HumanOnly => GuardMode::ReviewRequired,
                    other => other,
                };
            }

            match mode {
                GuardMode::Protected | GuardMode::Frozen | GuardMode::HumanOnly => {
                    return Decision::Rejected(format!("Target is protected (mode={:?}): {:?}", mode, op.file));
                }
                GuardMode::ReviewRequired => {
                    return Decision::ReviewRequired(format!("Target requires review: {:?}", op.file));
                }
                GuardMode::ProposalOnly => {
                    return Decision::ProposalOnly(format!("Target is proposal-only: {:?}", op.file));
                }
                _ => {}
            }

            // Lock-first-lines check
            if let Some(locked_count) = config.lock_first_lines {
                if op.old_range.start <= locked_count {
                    return Decision::Rejected(format!(
                        "Patch affects locked first {} lines in {:?}",
                        locked_count, op.file
                    ));
                }
            }

            // Marker region checks
            for marker in &markers {
                if Self::overlaps(op, marker.start_line, marker.end_line) {
                    if Self::touches_boundary(op, marker.start_line, marker.end_line) {
                        return Decision::Rejected(format!(
                            "Patch modifies guard marker boundary {:?} in {:?}",
                            marker.id, op.file
                        ));
                    }
                    match marker.mode {
                        GuardMode::Protected | GuardMode::Frozen | GuardMode::HumanOnly => {
                            return Decision::Rejected(format!(
                                "Patch affects locked region {:?} in {:?}",
                                marker.id, op.file
                            ));
                        }
                        _ => {}
                    }
                }
            }

            // Markdown section checks (global config + per-document inline policy)
            for section in &sections {
                if Self::overlaps(op, section.start_line, section.end_line) {
                    let locked_by_config = config.lock_sections.iter().any(|s| s == &section.title);
                    let locked_by_inline = inline_policy.locked.iter().any(|s| s == &section.title);
                    let editable_by_inline = inline_policy.editable.iter().any(|s| s == &section.title);
                    if (locked_by_config || locked_by_inline) && !editable_by_inline {
                        return Decision::Rejected(format!(
                            "Patch affects locked section {:?} in {:?}",
                            section.title, op.file
                        ));
                    }
                }
            }

            // Symbol registry checks (3.5 / 3.6)
            if let Some(reg) = symbol_registry {
                for line_offset in 0..op.old_range.count {
                    let line = op.old_range.start + line_offset;
                    if let Some(symbol) = reg.find_by_range(&op.file, line) {
                        // Whole-symbol lock
                        if config.lock_symbols.iter().any(|s| s == &symbol.name) {
                            return Decision::Rejected(format!(
                                "Patch affects locked symbol {:?} in {:?}",
                                symbol.name, op.file
                            ));
                        }

                        // Signature-only lock
                        if config.lock_signatures.iter().any(|s| s == &symbol.name) {
                            if let Some(body_start) = symbol.body_start_line {
                                if line < body_start {
                                    return Decision::Rejected(format!(
                                        "Patch affects locked signature of {:?} in {:?}",
                                        symbol.name, op.file
                                    ));
                                }
                            } else {
                                return Decision::Rejected(format!(
                                    "Patch affects locked signature (whole symbol) {:?} in {:?}",
                                    symbol.name, op.file
                                ));
                            }
                        }
                    }
                }
            }
        }

        Decision::Allowed
    }

    /// Verify a structured JSON patch against policy.
    pub fn verify_structured_patch(
        config: &Config,
        patch: &guardpatch_patch::StructuredPatch,
        _symbol_registry: Option<&SymbolRegistry>,
        active_unlocks: &[String],
    ) -> Decision {
        for op in &patch.operations {
            match op {
                guardpatch_patch::StructuredOperation::ReplaceFile { file, .. } => {
                    let mode = config.resolve_path_mode(file);
                    let file_str = file.to_str().unwrap_or("");
                    if matches!(mode, GuardMode::Protected | GuardMode::Frozen | GuardMode::HumanOnly)
                        && !Self::is_unlocked(file_str, active_unlocks)
                    {
                        return Decision::Rejected(format!("File is protected: {:?}", file));
                    }
                }
                guardpatch_patch::StructuredOperation::ReplaceSymbolBody { file, symbol_name, .. } => {
                    if config.lock_symbols.iter().any(|s| s == symbol_name) {
                        return Decision::Rejected(format!("Symbol {:?} is locked in {:?}", symbol_name, file));
                    }
                    // Signature is preserved by definition for ReplaceSymbolBody
                }
                guardpatch_patch::StructuredOperation::AppendSection { file, section_title, .. } => {
                    if config.lock_sections.iter().any(|s| s == section_title) {
                        return Decision::Rejected(format!("Section {:?} is locked in {:?}", section_title, file));
                    }
                }
                guardpatch_patch::StructuredOperation::DeleteFile { file } => {
                    let mode = config.resolve_path_mode(file);
                    if matches!(mode, GuardMode::Protected | GuardMode::Frozen) {
                        return Decision::Rejected(format!("Cannot delete protected file: {:?}", file));
                    }
                }
                _ => {}
            }
        }
        Decision::Allowed
    }

    fn is_unlocked(file_path: &str, active_unlocks: &[String]) -> bool {
        active_unlocks.iter().any(|u| u == file_path || file_path.starts_with(u.as_str()))
    }

    fn overlaps(op: &PatchOperation, start: usize, end: usize) -> bool {
        let op_start = op.old_range.start;
        let op_end = op.old_range.start + op.old_range.count.max(1) - 1;
        op_start <= end && op_end >= start
    }

    fn touches_boundary(op: &PatchOperation, start: usize, end: usize) -> bool {
        let op_start = op.old_range.start;
        let op_end = op.old_range.start + op.old_range.count.max(1) - 1;
        op_start <= start || op_end >= end
    }

    /// Classify which named regions an operation touches.
    /// Region names: "metadata", "public-interface", "implementation", "internal-helpers".
    fn classify_regions_for_op(
        op: &PatchOperation,
        file_content: Option<&str>,
        locs_range: Option<(usize, usize)>,
    ) -> Vec<String> {
        let mut regions = Vec::new();

        // LOCS block / frontmatter = "metadata" region
        if let Some((start, end)) = locs_range {
            if Self::overlaps(op, start, end) {
                regions.push("metadata".to_string());
            }
        }

        // AST-based classification: exported → "public-interface", private → "implementation"
        if let Some(content) = file_content {
            if let Some(adapter) = ParserRegistry::get_adapter(Path::new(&op.file)) {
                if let Ok(symbols) = adapter.parse_symbols(content) {
                    let mut touched_exported = false;
                    let mut touched_private = false;
                    for sym in &symbols {
                        if Self::overlaps(op, sym.start_line, sym.end_line) {
                            if sym.is_exported {
                                touched_exported = true;
                            } else {
                                touched_private = true;
                            }
                        }
                    }
                    if touched_exported {
                        regions.push("public-interface".to_string());
                    }
                    if touched_private {
                        regions.push("implementation".to_string());
                        regions.push("internal-helpers".to_string());
                    }
                }
            }
        }

        // Fallback: if no region identified, treat as "implementation"
        if regions.is_empty() && file_content.is_some() {
            regions.push("implementation".to_string());
        }

        regions
    }

    fn is_weakening(old: &guardpatch_locs::LocsMetadata, new: &guardpatch_locs::LocsMetadata) -> bool {
        if let (Some(old_g), Some(new_g)) = (&old.guard, &new.guard) {
            if let (Some(old_m), Some(new_m)) = (&old_g.mode, &new_g.mode) {
                if Self::mode_priority(old_m) > Self::mode_priority(new_m) {
                    return true;
                }
            }
        }
        false
    }

    fn mode_priority(mode: &GuardMode) -> u32 {
        match mode {
            GuardMode::Frozen => 100,
            GuardMode::Protected => 90,
            GuardMode::HumanOnly => 85,
            GuardMode::ReviewRequired => 70,
            GuardMode::ProposalOnly => 60,
            GuardMode::AppendOnly => 50,
            GuardMode::Editable => 30,
            GuardMode::Generated => 20,
            GuardMode::Deprecated => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guardpatch_policy::{Config, ProjectConfig, PathRule, GuardMode};
    use guardpatch_patch::{PatchOperation, Range, PatchLine};

    fn make_op(file: &str, start: usize, count: usize, lines: Vec<PatchLine>) -> PatchOperation {
        PatchOperation {
            file: std::path::PathBuf::from(file),
            old_range: Range { start, count },
            new_range: Range { start, count: lines.iter().filter(|l| !matches!(l, PatchLine::Remove(_))).count() },
            lines,
        }
    }

    fn editable_config() -> Config {
        Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            ..Default::default()
        }
    }

    // Phase 3.11 — AST-aware governance fixture tests

    #[test]
    fn test_p3_allowed_edit_to_unprotected_file() {
        let config = editable_config();
        let op = make_op("src/utils.ts", 10, 1, vec![
            PatchLine::Remove("const x = 1;".to_string()),
            PatchLine::Add("const x = 2;".to_string()),
        ]);
        let d = Verifier::verify_patch(&config, &[op], None, None, None, &[]);
        assert_eq!(d, Decision::Allowed);
    }

    #[test]
    fn test_p3_rejected_edit_to_protected_path() {
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            paths: vec![PathRule { pattern: "src/auth/**".to_string(), mode: GuardMode::Protected }],
            ..Default::default()
        };
        let op = make_op("src/auth/login.ts", 5, 1, vec![PatchLine::Add("// changed".to_string())]);
        let d = Verifier::verify_patch(&config, &[op], None, None, None, &[]);
        assert!(matches!(d, Decision::Rejected(_)));
    }

    #[test]
    fn test_p3_unlock_allows_protected_path() {
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            paths: vec![PathRule { pattern: "src/auth/**".to_string(), mode: GuardMode::Protected }],
            ..Default::default()
        };
        let op = make_op("src/auth/login.ts", 5, 1, vec![PatchLine::Add("// changed".to_string())]);
        // With the file in active_unlocks, it should be downgraded to ReviewRequired
        let d = Verifier::verify_patch(&config, &[op], None, None, None, &["src/auth/login.ts".to_string()]);
        assert!(matches!(d, Decision::ReviewRequired(_)));
    }

    #[test]
    fn test_p3_locked_symbols_reject_whole_symbol_edit() {
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            lock_symbols: vec!["calculateRiskScore".to_string()],
            ..Default::default()
        };
        let mut reg = SymbolRegistry::new();
        reg.register_file_symbols(
            std::path::PathBuf::from("src/risk.ts"),
            vec![guardpatch_parse::SymbolNode {
                id: "calculateRiskScore:0".to_string(),
                kind: guardpatch_parse::SymbolKind::Function,
                name: "calculateRiskScore".to_string(),
                start_line: 10,
                body_start_line: Some(11),
                end_line: 30,
                is_exported: true,
            }],
        );
        let op = make_op("src/risk.ts", 15, 5, vec![PatchLine::Add("return 0;".to_string())]);
        let d = Verifier::verify_patch(&config, &[op], None, Some(&reg), None, &[]);
        assert!(matches!(d, Decision::Rejected(_)));
    }

    #[test]
    fn test_p3_signature_lock_allows_body_edit() {
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            lock_signatures: vec!["processPayment".to_string()],
            ..Default::default()
        };
        let mut reg = SymbolRegistry::new();
        reg.register_file_symbols(
            std::path::PathBuf::from("src/payment.ts"),
            vec![guardpatch_parse::SymbolNode {
                id: "processPayment:0".to_string(),
                kind: guardpatch_parse::SymbolKind::Function,
                name: "processPayment".to_string(),
                start_line: 5,
                body_start_line: Some(7), // body starts at line 7
                end_line: 20,
                is_exported: true,
            }],
        );
        // Edit inside body (line 10) — should be allowed
        let op = make_op("src/payment.ts", 10, 1, vec![PatchLine::Add("// new logic".to_string())]);
        let d = Verifier::verify_patch(&config, &[op], None, Some(&reg), None, &[]);
        assert_eq!(d, Decision::Allowed);
    }

    #[test]
    fn test_p3_test_weakening_detected() {
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            detect_test_weakening: true,
            ..Default::default()
        };
        let content = "it('does x', () => {\n  assert.equal(result, 42);\n});\n";
        let op = make_op("src/payment.test.ts", 2, 1, vec![
            PatchLine::Remove("  assert.equal(result, 42);".to_string()),
        ]);
        let d = Verifier::verify_patch(&config, &[op], Some(content), None, None, &[]);
        assert!(matches!(d, Decision::ReviewRequired(_)));
    }

    // Phase 4 integration tests

    #[test]
    fn test_p4_active_unlocks_degrade_protected_to_review() {
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Protected,
                locs_required_for_new_files: false,
            },
            ..Default::default()
        };
        let op = make_op("src/core.ts", 5, 1, vec![PatchLine::Add("// edit".to_string())]);
        let d_before = Verifier::verify_patch(&config, &[op.clone()], None, None, None, &[]);
        assert!(matches!(d_before, Decision::Rejected(_)));

        let d_after = Verifier::verify_patch(&config, &[op], None, None, None, &["src/core.ts".to_string()]);
        assert!(matches!(d_after, Decision::ReviewRequired(_)));
    }

    // Phase 5 integration tests

    #[test]
    fn test_p5_agent_denied_outside_allow_list() {
        use guardpatch_policy::AgentProfile;
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            agents: vec![AgentProfile {
                name: "frontend_agent".to_string(),
                allow: vec!["src/ui/**".to_string()],
                deny: vec![],
                default_mode: None,
                proposal_only: false,
            }],
            ..Default::default()
        };
        let op = make_op("src/auth/session.ts", 1, 1, vec![PatchLine::Add("// hack".to_string())]);
        let d = Verifier::verify_patch(&config, &[op], None, None, Some("frontend_agent"), &[]);
        assert!(matches!(d, Decision::Rejected(_)));
    }

    // Phase 6 — inline region policy tests

    #[test]
    fn test_p6_locs_evidence_required_returns_review() {
        let config = editable_config();
        // File with guard.evidence_required set
        let content = r#"/*
LOCS:
  capability: payment
  stability: active
  guard:
    mode: editable
    evidence_required:
      - tests-pass
      - typecheck-pass
*/
pub fn process() {}
"#;
        let op = make_op("src/payment.ts", 11, 1, vec![
            PatchLine::Add("// impl change".to_string()),
        ]);
        let d = Verifier::verify_patch(&config, &[op], Some(content), None, None, &[]);
        assert!(matches!(d, Decision::ReviewRequired(_)), "expected ReviewRequired, got {:?}", d);
    }

    #[test]
    fn test_p6_locs_locked_region_metadata_rejects_edit_to_locs_block() {
        let config = editable_config();
        let content = r#"/*
LOCS:
  capability: auth
  stability: stable
  guard:
    mode: editable
    locked_regions:
      - metadata
      - public-interface
*/
pub fn login() {}
"#;
        // Patch touches line 2 — inside the LOCS block (metadata region)
        let op = make_op("src/auth.ts", 2, 1, vec![
            PatchLine::Remove("  capability: auth".to_string()),
            PatchLine::Add("  capability: other".to_string()),
        ]);
        let d = Verifier::verify_patch(&config, &[op], Some(content), None, None, &[]);
        assert!(matches!(d, Decision::Rejected(_)), "expected Rejected for metadata edit, got {:?}", d);
    }

    #[test]
    fn test_p6_locs_editable_region_allows_impl_edit_in_protected_file() {
        let config = Config {
            project: guardpatch_policy::ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Protected,
                locs_required_for_new_files: false,
            },
            ..Default::default()
        };
        // File is globally Protected but declares implementation as editable
        let content = r#"/*
LOCS:
  capability: utils
  guard:
    mode: protected
    editable_regions:
      - implementation
      - internal-helpers
*/
fn helper() { 1 + 1 }
"#;
        // Line 10 is inside the private helper function — "implementation" region
        let op = make_op("src/utils.ts", 10, 1, vec![
            PatchLine::Add("// allowed impl change".to_string()),
        ]);
        let d = Verifier::verify_patch(&config, &[op], Some(content), None, None, &[]);
        // Should be allowed because the touched region is in editable_regions
        assert_eq!(d, Decision::Allowed, "expected Allowed for impl edit in editable_regions, got {:?}", d);
    }

    #[test]
    fn test_p5_proposal_only_agent_returns_proposal_decision() {
        use guardpatch_policy::AgentProfile;
        let config = Config {
            project: ProjectConfig {
                name: "test".to_string(),
                mode: GuardMode::Editable,
                locs_required_for_new_files: false,
            },
            agents: vec![AgentProfile {
                name: "doc_agent".to_string(),
                allow: vec!["docs/**".to_string()],
                deny: vec![],
                default_mode: None,
                proposal_only: true,
            }],
            ..Default::default()
        };
        let op = make_op("docs/readme.md", 1, 1, vec![PatchLine::Add("new content".to_string())]);
        let d = Verifier::verify_patch(&config, &[op], None, None, Some("doc_agent"), &[]);
        assert!(matches!(d, Decision::ProposalOnly(_)));
    }
}
