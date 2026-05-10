# GuardPatch

GuardPatch is a deterministic permission and verification layer for AI-generated edits.

> LLMs propose changes. GuardPatch enforces who may change what.

## What it does

GuardPatch intercepts patch proposals before they are applied. It validates each change against a policy file (`.guardpatch.yml`), LOCS metadata, and the project's AST structure. A patch is only applied if all checks pass.

## Features

| Phase | Feature |
|---|---|
| 1 | File/path protection, line-range locks, marker regions, Markdown section locks |
| 2 | LOCS metadata governance, weakening detection, new-file LOCS requirement |
| 3 | TypeScript/JavaScript and Python AST parsing, symbol/signature/export locking, test-weakening detection, structured patch format |
| 4 | Evidence-gated promotion lifecycle, unlock registry, git history checks, relock workflow, audit report |
| 5 | Per-agent authority profiles, proposal-only mode, review queue, patch risk scoring, stdin JSON adapter |
| 6 | Per-file inline region policy (locked/editable regions, evidence/approval requirements), inline Markdown governance, evidence ledger |

## Quick start

```bash
# Initialise a project
guardpatch init

# Verify a patch
guardpatch verify patch.diff

# Apply a verified patch
guardpatch apply patch.diff

# Verify a structured JSON patch from stdin (LLM tool adapter)
echo '{"operations":[...]}' | guardpatch verify --stdin-json

# Show protected surfaces
guardpatch scan
```

## Agent-aware verification

Add agent profiles to `.guardpatch.yml`:

```yaml
agents:
  - name: frontend_agent
    allow:
      - "src/ui/**"
      - "docs/**"
    deny:
      - "src/auth/**"
    proposal_only: false

  - name: doc_agent
    allow:
      - "docs/**"
    proposal_only: true
```

Then verify with an actor flag:

```bash
guardpatch verify patch.diff --actor frontend_agent
```

## Unlock workflow (Phase 4)

```bash
# Unlock a protected target for one patch
guardpatch unlock src/core/risk.ts \
  --reason "new enterprise pricing model" \
  --scope one_patch

# Apply the patch (unlock is consumed)
guardpatch apply patch.diff

# Or relock manually
guardpatch relock src/core/risk.ts
```

## Promotion lifecycle (Phase 4)

```bash
# Promote a file to stable after evidence checks
guardpatch promote src/core/risk.ts --to stable --evidence tests,typecheck

# Full lifecycle
guardpatch promote src/core/risk.ts --to protected --evidence tests,user_approval
```

## Review queue (Phase 5)

```bash
# List patches awaiting review
guardpatch review list

# Approve a queued patch
guardpatch review approve <id>

# Reject with reason
guardpatch review reject <id> --reason "security concern"
```

## Audit log

```bash
guardpatch audit           # Recent entries
guardpatch audit --report  # Summary table with counts
```

## Evidence ledger (Phase 6)

Every accepted patch is recorded in `.guardpatch/ledger.jsonl`:

```json
{
  "change_id": "a1b2c3d4-...",
  "timestamp": "2026-05-10T12:00:00Z",
  "module_id": "src/core/risk.ts",
  "changed_regions": ["src/core/risk.ts"],
  "policy_result": "allowed",
  "tests_passed": null,
  "typecheck_passed": null,
  "human_approval": false,
  "risk_score": 18,
  "actor": "frontend_agent"
}
```

```bash
guardpatch ledger           # Recent ledger entries (table)
guardpatch ledger --json    # Full JSON output
guardpatch ledger --limit 5 # Last 5 entries
```

## Per-file inline region policy (Phase 6)

Declare editable and locked regions directly in each file's LOCS metadata:

```typescript
/*
 * LOCS:
 *   capability: payment-processor
 *   stability: stable
 *   guard:
 *     mode: review_required
 *     locked_regions:
 *       - metadata
 *       - public-interface
 *       - behaviour-contract
 *     editable_regions:
 *       - implementation
 *       - internal-helpers
 *       - example-usage
 *     approval_required:
 *       - interface-change
 *       - dependency-change
 *     evidence_required:
 *       - tests-pass
 *       - typecheck-pass
 */
```

Named regions for code files:

| Region name | Covers |
|---|---|
| `metadata` | LOCS frontmatter or block-comment (auto-detected line range) |
| `public-interface` | Exported functions and classes (AST-detected) |
| `implementation` | Non-exported functions |
| `internal-helpers` | Private/internal helpers (same as implementation) |
| `behaviour-contract` | Symbols listed in `lock_symbols` |

For Markdown files, region names are section titles.

**How it works:**
- `locked_regions` — any patch touching a locked region is `Rejected`
- `editable_regions` — if all touched regions are editable, the edit is `Allowed` even if the file's overall mode is `protected`
- `evidence_required` — any patch to this file returns `ReviewRequired` until evidence is confirmed via `guardpatch promote --evidence`

## Per-document Markdown governance (Phase 6)

Declare section locks inline without touching `.guardpatch.yml`:

```markdown
<!-- guardpatch-locked: Title, Core Principles, Metadata Schema -->
<!-- guardpatch-editable: Examples, Implementation Notes, Changelog -->
```

These override or supplement the global `lock_sections` list. An `editable` declaration takes precedence over a `locked-by-config` rule for the same section.

## Policy file (.guardpatch.yml)

```yaml
version: 1

project:
  name: my-project
  mode: editable
  locs_required_for_new_files: true

paths:
  - pattern: "src/auth/**"
    mode: protected
  - pattern: "src/core/**"
    mode: review_required

lock_sections:
  - "Core Invariants"
  - "Runtime Model"

lock_symbols:
  - "calculateRiskScore"

lock_signatures:
  - "processPayment"

lock_exports: true
lock_dependencies: true
detect_test_weakening: true

patch_limits:
  max_files_changed: 10
  max_lines_changed: 500
  dependency_changes_require_approval: true

unlock_policy:
  require_reason: true
  auto_relock_after_merge: true

agents:
  - name: frontend_agent
    allow:
      - "src/ui/**"
    deny:
      - "src/auth/**"
```

## Architecture

```
crates/
  guardpatch-policy/    # Config, GuardMode, AgentProfile, PatchLimits
  guardpatch-locs/      # LOCS metadata extractor, GuardConfig (region fields), find_metadata_line_range
  guardpatch-parse/     # Parser registry, TypeScript/Python AST adapters, markers, Markdown, InlineMarkdownPolicy
  guardpatch-patch/     # Unified diff parser, patch applier, structured patch format
  guardpatch-core/      # Verifier, SymbolRegistry, classify_regions_for_op, risk_score
  guardpatch-lifecycle/ # PromotionStore, UnlockRegistry, EvidenceRunner, ReviewQueue
  guardpatch-audit/     # VerificationReport, AuditStore, ChangeLedgerEntry, EvidenceLedger
  guardpatch-cli/       # CLI entry point (all commands)
```

## Test suite

```
cargo test
```

33 tests across all crates covering Phase 3–6 scenarios:
- Protected path rejection / unlock degradation
- Symbol and signature locking
- Test weakening detection
- Agent authority enforcement
- Proposal-only mode
- Review queue lifecycle
- Risk score computation
- Promotion state ordering
- Unlock scope semantics
- LOCS `locked_regions` enforcement (metadata, public-interface)
- LOCS `editable_regions` early-allow override
- LOCS `evidence_required` → ReviewRequired
- `find_metadata_line_range` (frontmatter + block comment)
- `GuardConfig` region-field YAML round-trip
- Inline Markdown policy (`<!-- guardpatch-locked/editable: ... -->`)
