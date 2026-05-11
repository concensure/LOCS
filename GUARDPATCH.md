# GuardPatch

**Enables Reliable Autonomous Coding.**

GuardPatch is a deterministic permission system for AI coding agents. It is a policy layer that controls which parts of a project may be edited, proposed, reviewed, or frozen — sitting between the LLM and the codebase and enforcing machine-verifiable boundaries on every change.

> LLMs propose. GuardPatch decides.

---

## The Problem No One Has Solved

Search GitHub for "AI edit governance", "LLM patch validation", or "safe autonomous coding". You will find linters, formatters, and post-hoc review tools. You will not find a tool that intercepts AI-generated edits *before they are applied* and enforces who may change what, at the region and symbol level, without human attention for every patch.

The gap exists because most AI coding tools treat the LLM as the terminal authority. The LLM writes directly to files. There is no enforcement layer between intent and disk.

This works for toy demos. It does not work for production codebases, validated specifications, or any project where drift, deletion, or overreach has a real cost.

---

## Why LLMs Fail at Bounded Editing

LLMs are probabilistic systems asked to make surgical, deterministic changes. The mismatch produces four recurring failure modes:

**1. Scope overreach.** Asked to update one function, the model rewrites the entire file. Asked to add an example, it restructures the document. The change is plausible but far exceeds what was requested.

**2. Structure drift.** Headers move. Section ordering changes. Semantic labels shift. The document remains readable but loses its machine-parseable structure and breaks downstream tooling.

**3. Hallucinated deletion.** The model omits a validated invariant, a critical warning, or a carefully authored note — not to improve it, but because it did not notice it was there. These deletions are invisible in a cursory review.

**4. Metadata corruption.** Governance metadata (LOCS annotations, stability labels, ownership fields) gets rewritten to match the narrative the model is constructing, not the ground truth the metadata was encoding.

Markdown amplifies all four failures because it is free-form text with no schema enforcement and no native concept of partial mutability. A document has no machine-readable way to say "this section is fixed; that section is yours to edit."

The goal is to give every file that property: **allow AI to generate content, but only within strict, machine-verifiable boundaries**.

---

## The Architecture

```
[User Intent]
     ↓
[LLM generates PATCH or PROPOSAL]
     ↓
[GuardPatch Deterministic Verifier]
     ↓
[Accept / ReviewRequired / Reject / ProposalOnly]
     ↓
[Write to file  —  or —  Queue for review]
```

The key shift: **the LLM does not directly edit files. It proposes a patch. GuardPatch decides whether to apply it.**

This single constraint — interposing a deterministic verifier between generation and application — is what makes reliable autonomous coding possible. The LLM retains full generative capability. The verifier retains full enforcement authority. Neither compromises the other.

---

## What GuardPatch Does

### Path and file protection

Protect entire directories or specific files using glob patterns. Changes to protected paths are rejected outright. Changes to review-required paths are queued for human approval.

```yaml
paths:
  - pattern: "src/auth/**"
    mode: protected
  - pattern: "src/core/**"
    mode: review_required
```

### Per-region governance inside files

A file is not a monolith. GuardPatch understands internal structure and enforces policy at the region level, not just the file level.

**For code files** (TypeScript, JavaScript, Python — parsed with tree-sitter):

| Region | Covers |
|---|---|
| `metadata` | LOCS annotation block or frontmatter — the governance contract itself |
| `public-interface` | Exported functions and classes — the API surface |
| `behaviour-contract` | Symbols explicitly listed in `lock_symbols` |
| `implementation` | Non-exported functions — the safe zone for most edits |
| `internal-helpers` | Private helpers — editable by default |

**For Markdown files** — regions are section titles. Lock them globally in `.guardpatch.yml` or inline per-document:

```markdown
<!-- guardpatch-locked: Core Principles, Metadata Schema, System Architecture -->
<!-- guardpatch-editable: Examples, Implementation Notes, Changelog -->
```

Declare region policy directly in the file's LOCS metadata for per-file precision:

```
/*
 * LOCS:
 *   capability: payment-processor
 *   stability: stable
 *   guard:
 *     mode: review_required
 *     locked_regions:
 *       - metadata
 *       - public-interface
 *     editable_regions:
 *       - implementation
 *       - internal-helpers
 *     evidence_required:
 *       - tests-pass
 *       - typecheck-pass
 */
```

### AST-backed symbol enforcement

GuardPatch parses the file's AST before and after applying each patch:

- `lock_symbols` — reject any change that touches named functions or classes
- `lock_signatures` — lock the function signature but allow body edits
- `lock_exports` — reject changes that remove or unexport any public API symbol
- `lock_dependencies` — flag new imports or direct edits to dependency manifests
- `detect_test_weakening` — flag removal of `assert` / `expect` statements or addition of `skip` / `ignore`

### LOCS metadata integration

GuardPatch reads [LOCS](README.md) annotations embedded in files. If a file's LOCS metadata declares `mode: protected`, that overrides the path rule. If a patch would weaken the governance level of a file's metadata (e.g. changing `protected` → `editable`), the patch is rejected before it reaches disk.

New files added to a project can be required to carry LOCS metadata:

```yaml
project:
  locs_required_for_new_files: true
```

### Per-agent authority profiles

Different agents have different authority. GuardPatch enforces this explicitly:

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
    proposal_only: true     # can propose, never apply directly
```

Run verification with an actor:

```bash
guardpatch verify patch.diff --actor frontend_agent
```

An agent operating outside its allowed paths is rejected. An agent marked `proposal_only` can never write directly — every change goes to the review queue first.

### Evidence-gated promotion lifecycle

Files move through a stability lifecycle. Promotion from one state to the next requires evidence:

```
draft → active → stabilising → stable → protected → frozen
```

```bash
guardpatch promote src/core/risk.ts --to stable --evidence tests,typecheck
guardpatch promote src/core/risk.ts --to protected --evidence tests,user_approval
```

Evidence checks run automatically (`cargo test`, `npm test`, `pytest`, `tsc --noEmit`, `cargo check` — auto-detected by project type). A file cannot be promoted unless the checks pass.

**Mandatory evidence floor:** promoting to `protected` or `frozen` always requires at least one evidence item, regardless of `.guardpatch.yml` configuration. This is a hard constraint, not an opt-in. Attempting to promote without evidence will fail:

```
Error: Promoting 'src/core/risk.ts' to Protected requires at least one evidence item
       (e.g. --evidence tests,typecheck or --evidence user_approval).
       These stability levels enforce an immutable audit trail.
```

### The unlock workflow

Protected does not mean permanently immutable. It means: *this cannot be casually changed by an AI agent without an explicit unlock*.

```
Protected file
     ↓
User requests change with a stated reason
     ↓
System records the unlock with scope (one patch / branch / time-limited)
     ↓
LLM proposes change
     ↓
GuardPatch verifier checks impact (downgraded to review_required)
     ↓
User approves via review queue
     ↓
Change is applied, unlock is consumed, file returns to protected
```

```bash
# Unlock for a single patch
guardpatch unlock src/core/risk.ts \
  --reason "new enterprise pricing tier" \
  --scope one_patch

# Apply (unlock is consumed automatically)
guardpatch apply patch.diff

# Or relock manually without applying
guardpatch relock src/core/risk.ts
```

Scopes: `one_patch` | `branch` | `time_limited` | `review_required`

### Review queue

Patches that require review are not discarded — they are queued:

```bash
guardpatch review list
guardpatch review approve <id>
guardpatch review reject <id> --reason "security concern"
```

The queue persists to `.guardpatch/review_queue.jsonl`. Each item records the patch reference, the reason it required review, and the actor who submitted it.

### Evidence ledger

Every accepted patch is recorded to `.guardpatch/ledger.jsonl`:

```json
{
  "change_id": "a1b2c3d4-...",
  "timestamp": "2026-05-10T12:00:00Z",
  "module_id": "src/core/risk.ts",
  "changed_regions": ["src/core/risk.ts"],
  "policy_result": "allowed",
  "risk_score": 18,
  "actor": "frontend_agent"
}
```

```bash
guardpatch ledger           # table view
guardpatch ledger --json    # machine-readable
```

### Audit log rotation

The audit log (`.guardpatch/audit.jsonl`) grows unboundedly. Rotate it to archive the current file and start fresh:

```bash
# Rotate when log exceeds 10 000 entries (default)
guardpatch audit rotate

# Rotate when log exceeds a custom threshold
guardpatch audit rotate --max-entries 5000

# Rotate unconditionally regardless of size
guardpatch audit rotate --force
```

Rotation archives the current log to `.guardpatch/audit.YYYY-MM-DD.jsonl` (with a numeric suffix if that name is already taken) and starts a fresh log on the next verification. The archive is a plain JSONL file and can be inspected with standard tools.

### Patch risk scoring

Every patch receives a deterministic risk score (0–100) based on files changed, lines changed, and protected symbols touched. The score is available via `guardpatch explain patch.diff` and is recorded in the ledger.

### Structured rejection output

Every rejected or review-required decision includes a machine-readable fix hint and the rule source so developers and agent frameworks know exactly what to do next:

```
--- GuardPatch Report ---
Status:   Rejected("Target is protected (mode=Protected): \"src/auth/login.ts\"")
Summary:  Patch rejected: Target is protected (mode=Protected): "src/auth/login.ts"
Fix:      Run: guardpatch unlock src/auth/login.ts --reason "<reason>" --scope one_patch
Rule:     path protection rule (.guardpatch.yml paths[])
Files:    ["src/auth/login.ts"]
Lines:    2
-------------------------
```

The `Fix:` line is a ready-to-run command. The `Rule:` line identifies the exact policy that triggered the decision. Both fields are also present in the JSON output (`--json`):

```json
{
  "fix_hint": "Run: guardpatch unlock src/auth/login.ts --reason \"<reason>\" --scope one_patch",
  "rule_source": "path protection rule (.guardpatch.yml paths[])"
}
```

### LLM tool adapter

For integration with AI coding tools that generate structured edits:

```bash
echo '{"operations":[...]}' | guardpatch verify --stdin-json
```

The structured patch format (`ReplaceSymbolBody`, `AppendSection`, `ReplaceFile`, `DeleteFile`) gives LLMs a vocabulary for precise, region-targeted edits instead of raw unified diffs — and allows the verifier to enforce symbol-level and section-level constraints without line-number fragility.

---

## Token Efficiency (Underrated Benefit)

Because GuardPatch defines a precise mutable surface area for every file, an LLM integration can send only the editable regions — not the entire file — as context, and receive back only a structured patch for those regions.

In a moderately sized project with stable core logic and frequently iterated implementation details, this translates to 40–80% token reduction per edit cycle. On a project with daily LLM-assisted development, this compounds into meaningful cost and latency savings.

---

## Defining a Mutable Surface Area

GuardPatch introduces the concept of a **mutable surface area** for AI systems — the precise set of locations in a project that a given agent, at a given time, may propose changes to. This surface is bounded by:

- **Spatial boundaries** — file paths, line ranges, marker regions
- **Semantic boundaries** — AST nodes (functions, classes, exports, imports)
- **Structural boundaries** — Markdown sections, document headings
- **Operational boundaries** — what actions are permitted (edit / propose / review / freeze)

The surface area is not fixed. It changes as files are promoted, unlocks are granted, agents are configured, and evidence requirements are met. GuardPatch tracks all of this state deterministically.

---

## Bridging Probabilistic and Deterministic Systems

AI coding tools occupy an awkward middle ground: they use probabilistic generation (the LLM) to produce what should be deterministic, correct edits. The mismatch is the source of most failure modes.

GuardPatch resolves this by making the boundary explicit:

```
Probabilistic zone     →    [LLM]     →   generates patch proposals
────────────────────────────────────────────────────────────────────
Deterministic zone     →  [GuardPatch]  →  enforces policy, decides
────────────────────────────────────────────────────────────────────
Ground truth           →   [files]    →   only accepts verified writes
```

The LLM operates with full generative freedom inside the probabilistic zone. GuardPatch operates with full enforcement authority in the deterministic zone. Neither compromises the other. This is the missing layer in most AI coding tools today.

---

## Quick Start

```bash
# Install (Rust toolchain required)
cargo install --path guardpatch/crates/guardpatch-cli

# Initialise a project
guardpatch init

# Scan what is protected
guardpatch scan

# Verify a patch before applying
guardpatch verify patch.diff

# Verify and apply
guardpatch apply patch.diff

# Verify with an agent identity
guardpatch apply patch.diff --actor frontend_agent

# Pipe a structured JSON patch from an LLM tool
echo '{"operations":[...]}' | guardpatch verify --stdin-json

# Show recent audit log
guardpatch audit --report

# Rotate audit log (archive when > 10 000 entries)
guardpatch audit rotate

# Show applied-patch ledger
guardpatch ledger
```

---

## Policy File (`.guardpatch.yml`)

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
  - name: doc_agent
    allow:
      - "docs/**"
    proposal_only: true
```

---

## Architecture

```
crates/
  guardpatch-policy/    # Config, GuardMode, AgentProfile, PatchLimits
  guardpatch-locs/      # LOCS metadata parser, GuardConfig region fields, metadata line-range detection
  guardpatch-parse/     # TypeScript/Python AST adapters (tree-sitter), marker parser, Markdown section parser,
                        # InlineMarkdownPolicy (<!-- guardpatch-locked: ... --> syntax)
  guardpatch-patch/     # Unified diff parser, in-memory patch applier, structured patch format
  guardpatch-core/      # Verifier, SymbolRegistry, region classifier, risk scorer
  guardpatch-lifecycle/ # PromotionStore, UnlockRegistry, EvidenceRunner, ReviewQueue
  guardpatch-audit/     # VerificationReport, AuditStore, ChangeLedgerEntry, EvidenceLedger
  guardpatch-cli/       # CLI entry point — all commands
```

The crates have a strict dependency direction: `cli → audit → lifecycle → core → parse → patch → locs → policy`. No circular dependencies.

---

## Relationship to LOCS

LOCS and GuardPatch are complementary, not dependent.

[LOCS](README.md) is a metadata schema for annotating what a module *is* — its capability, stability, owner, and token cost. It makes a codebase machine-readable and efficiently routable for LLM retrieval.

GuardPatch is an enforcement layer for what a module *may have done to it* — who can edit it, under what conditions, and with what evidence. It reads LOCS metadata as one input to its policy decisions.

You can use LOCS without GuardPatch. You can use GuardPatch without LOCS (it will fall back to path-only and config-only governance). The combination gives you a codebase that is both efficiently navigable by AI agents and safely editable by them.

---

## Test Suite

```bash
cd guardpatch
cargo test
```

41 tests across all crates covering:

- Protected path rejection and unlock degradation
- Symbol locking and signature-only locking
- Export removal detection
- Dependency drift detection
- Test weakening heuristics
- Agent authority enforcement and proposal-only mode
- Promotion state ordering and evidence gating
- Unlock scope semantics (one_patch / branch / time_limited)
- Review queue lifecycle
- Risk score computation
- Per-file `locked_regions` and `editable_regions` enforcement
- `evidence_required` → ReviewRequired gate
- LOCS metadata line-range detection (frontmatter + block comment)
- Inline Markdown policy parsing (`<!-- guardpatch-locked/editable: ... -->`)
- GuardConfig region-field YAML round-trip
- Mandatory evidence floor for Protected and Frozen states
