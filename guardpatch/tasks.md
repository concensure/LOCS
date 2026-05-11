# GuardPatch Tasks

## Phase 0 — Foundation [DONE]
- [x] Create repository scaffold
- [x] Establish architecture

## Phase 1 — Deterministic Boundaries [DONE]
- [x] 1.1 Implement CLI command structure
- [x] 1.2 Implement config loader
- [x] 1.3 Implement glob/path rule matching
- [x] 1.4 Implement unified diff parser
- [x] 1.5 Implement in-memory patch application
- [x] 1.6 Implement line-range verifier
- [x] 1.7 Implement marker parser
- [x] 1.8 Implement marker-region verifier
- [x] 1.9 Implement Markdown section parser
- [x] 1.10 Implement Markdown section verifier
- [x] 1.11 Implement basic report generator
- [x] 1.12 Implement JSON decision output
- [x] 1.13 Implement audit log
- [x] 1.14 Implement apply command with verification gate
- [x] 1.15 Phase 1 fixture suite

## Phase 2 — LOCS-Aware Governance [DONE]
- [x] 2.1 Define LOCS metadata schema
- [x] 2.2 Implement Markdown frontmatter LOCS extractor
- [x] 2.3 Implement code comment LOCS extractor
- [x] 2.4 Integrate LOCS into policy resolver
- [x] 2.5 Implement LOCS weakening detection
- [x] 2.6 Implement new-file LOCS requirement
- [x] 2.7 Add LOCS templates
- [x] 2.8 Phase 2 fixture suite

## Phase 3 — AST-Aware Code Governance [DONE]
- [x] 3.1 Add parser registry (`ParserRegistry` in `guardpatch-parse`)
- [x] 3.2 Implement TypeScript/JavaScript AST adapter (tree-sitter)
- [x] 3.3 Implement Python AST adapter (tree-sitter)
- [x] 3.4 Implement symbol registry (`SymbolRegistry` in `guardpatch-core`)
- [x] 3.5 Implement function/class protection (verifier checks against registry)
- [x] 3.6 Implement signature locking (`lock_signatures` config + body_start_line heuristic)
- [x] 3.7 Implement exported API locking (`lock_exports` config)
- [x] 3.8 Implement import/dependency drift checks (`lock_dependencies` config)
- [x] 3.9 Implement test weakening heuristics (`detect_test_weakening` config)
- [x] 3.10 Implement structured patch format (`StructuredPatch`, `StructuredOperation`)
- [x] 3.11 Phase 3 fixture suite (12 tests in `guardpatch-core`: p3/p4/p5 verifier tests)

## Phase 4 — Evidence-Gated Stability Lifecycle [DONE]
- [x] 4.1 Implement promotion state model (`PromotionState`: draft→active→stabilising→stable→protected→frozen)
- [x] 4.2 Implement Git history checks (`GitHistoryChecker` using `git log`)
- [x] 4.3 Implement evidence command runner (`EvidenceRunner` auto-detects cargo/npm/pytest)
- [x] 4.4 Implement promotion command (`guardpatch promote <target> --to <state> --evidence tests,typecheck`)
- [x] 4.5 Implement unlock registry (`UnlockRegistry` persisted to `.guardpatch/unlocks.json`)
- [x] 4.6 Implement unlock command (`guardpatch unlock <target> --reason <r> --scope <s>`)
- [x] 4.7 Integrate unlock into verifier (active unlocks degrade Protected→ReviewRequired)
- [x] 4.8 Implement relock workflow (`guardpatch relock <target>`, auto-consume one_patch scope on apply)
- [x] 4.9 Implement audit report (`guardpatch audit --report` shows summary table)
- [x] 4.10 Phase 4 fixture suite (3 tests in `guardpatch-lifecycle`: promotion, unlock, review)

## Phase 5 — Agent-Aware Edit Authority [DONE]
- [x] 5.1 Define actor model (`ActorType` implicit via `--actor` CLI flag)
- [x] 5.2 Implement agent profile config (`AgentProfile` in `guardpatch-policy`, `agents:` section in YAML)
- [x] 5.3 Implement proposal-only mode (`proposal_only: true` on `AgentProfile`, returns `Decision::ProposalOnly`)
- [x] 5.4 Implement review queue (`ReviewQueue` in `guardpatch-lifecycle`, persisted to `.guardpatch/review_queue.jsonl`)
- [x] 5.5 Implement patch risk scoring (`risk_score::compute_score` in `guardpatch-core`)
- [x] 5.6 Add adapter mode for LLM coding tools (`guardpatch verify --stdin-json` reads structured JSON from stdin)
- [x] 5.7 Optional MCP/tool server adapter (deferred — CLI adapter mode covers the use case)
- [x] 5.8 Phase 5 fixture suite (2 agent-authority tests in `guardpatch-core`, 2 review queue tests in `guardpatch-lifecycle`)

## Phase 6 — Per-File Edit Policy + Evidence Ledger [DONE]

Dependencies: Phase 2 (LOCS schema), Phase 3 (AST parsing), Phase 4 (evidence runner)

- [x] 6.1 Extend `GuardConfig` in `guardpatch-locs` with inline region fields:
      `locked_regions`, `editable_regions`, `approval_required`, `evidence_required`
      (4 optional `Vec<String>` fields, YAML-serialisable; backward-compatible)
- [x] 6.2 Implement `LocsExtractor::find_metadata_line_range` — returns `(start, end)` line range
      of the LOCS frontmatter or block-comment so the verifier can classify the "metadata" region
- [x] 6.3 Implement `Verifier::classify_regions_for_op` — maps a patch operation to named regions
      ("metadata", "public-interface", "implementation", "internal-helpers") using the LOCS range
      and the AST adapter (exported vs. private symbols)
- [x] 6.4 Enforce `locked_regions` in verifier: if any touched region is in the file's
      `guard.locked_regions` list, return `Decision::Rejected`
- [x] 6.5 Enforce `editable_regions` early-allow: if all touched regions are in
      `guard.editable_regions`, return `Decision::Allowed` even when the file is otherwise Protected
- [x] 6.6 Enforce `evidence_required`: if the field is non-empty, return `Decision::ReviewRequired`
      prompting the user to supply evidence via `guardpatch promote --evidence`
- [x] 6.7 Add `InlineMarkdownPolicy` to `guardpatch-parse` — parse
      `<!-- guardpatch-locked: A, B -->` and `<!-- guardpatch-editable: X, Y -->` HTML comments
      from Markdown files; export as `InlineMarkdownPolicy { locked, editable }`
- [x] 6.8 Merge per-document inline policy with global `lock_sections` in verifier:
      inline-locked sections are rejected, inline-editable overrides locked-by-config
- [x] 6.9 Implement `ChangeLedgerEntry` in `guardpatch-audit`:
      `{ change_id (uuid), timestamp, module_id, changed_regions, policy_result, tests_passed,
         typecheck_passed, human_approval, risk_score, actor }`
- [x] 6.10 Implement `EvidenceLedger` in `guardpatch-audit` — persists to
      `.guardpatch/ledger.jsonl`; methods: `record`, `load_recent`
- [x] 6.11 Wire evidence ledger into `guardpatch apply` — on `Decision::Allowed`, compute
      risk score and write a `ChangeLedgerEntry` automatically
- [x] 6.12 Add `guardpatch ledger [--limit N] [--json]` CLI command to inspect the ledger
- [x] 6.13 Phase 6 fixture suite:
      - `test_p6_locs_evidence_required_returns_review` (guardpatch-core)
      - `test_p6_locs_locked_region_metadata_rejects_edit_to_locs_block` (guardpatch-core)
      - `test_p6_locs_editable_region_allows_impl_edit_in_protected_file` (guardpatch-core)
      - `test_find_metadata_line_range_frontmatter` (guardpatch-locs)
      - `test_find_metadata_line_range_block_comment` (guardpatch-locs)
      - `test_find_metadata_line_range_none_for_plain_file` (guardpatch-locs)
      - `test_guard_config_region_fields_round_trip` (guardpatch-locs)
      - `test_parse_inline_policy_locked_and_editable` (guardpatch-parse)
      - `test_parse_inline_policy_empty` (guardpatch-parse)
- [x] 6.14 Implement LOCS Section Addressing (Stable IDs, inline modes, and roles)
      - Updated `DocumentSection` and `MarkerRange` with `id`, `mode`, and `role`
      - Implemented regex-based anchor parsing for Markdown and Code
      - Integrated Section Addressing into `Verifier::verify_patch`
      - Updated classification to respect explicit `role` fields

## Phase 7 — Advanced Governance & Low-Friction Workflows [DONE]

- [x] 7.1 Implement "Ghost Inference" (Path-based role mapping)
- [x] 7.2 Implement "Shadow Policy" (Sidecar config loading)
- [x] 7.3 Implement "Evidence Mapping" (Linking roles to commands)
- [x] 7.4 Implement "Self-Stabilizing Headers" (`guardpatch seal` and `guardpatch unseal`)
- [x] 7.5 Implement "Policy Integrity" (Gating modification of `.guardpatch.yml`)
- [x] 7.6 Phase 7 unit tests (Policy integrity, Ghost inference)
