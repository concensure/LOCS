# GuardPatch Implementation Prompt

You are an expert software architect and senior implementation agent. Build GuardPatch as a standalone deterministic edit-governance CLI for LLM-assisted projects. Use the accompanying `DESIGN.md` and `tasks.md` as the source of truth.

Your objective is to implement GuardPatch from Phase 1 to Phase 5 as a standalone project that builds on top of LOCS, then prepare a later optional integration with Semantic CLI. Do not start with Semantic integration. GuardPatch must be valuable by itself first.

---

## 1. Product Definition

GuardPatch is a deterministic permission and verification layer for AI-generated edits.

The central rule is:

> LLMs propose patches. GuardPatch verifies and applies only authorised patches.

GuardPatch must prevent accidental or unauthorised LLM overwrites of protected project surfaces, including source code, Markdown, Markdoc, MDX, configuration files, prompts, memory files, architecture documents, LOCS metadata, and eventually AST-level code symbols.

---

## 2. Required Documents

Before implementing, read and follow:

1. `DESIGN.md` — architecture, concepts, verification model, policy model, roadmap.
2. `tasks.md` — dependency-ordered implementation plan.
3. `.guardpatch.yml` — project policy once created.

Do not improvise a different architecture unless the change is clearly superior, documented, and compatible with the design intent.

---

## 3. Development Strategy

Implement in phases:

1. Phase 1 — Deterministic Boundaries
2. Phase 2 — LOCS-Aware Governance
3. Phase 3 — AST-Aware Code Governance
4. Phase 4 — Evidence-Gated Stability Lifecycle
5. Phase 5 — Agent-Aware Edit Authority
6. Phase 6 — Later Semantic CLI integration preparation only

Do not implement Phase 6 before Phase 1–5 foundations are stable.

---

## 4. LOCS Requirement for New Files

When creating any new non-trivial file, reference LOCS.

Each meaningful source, config, parser, verifier, policy, CLI, report, or documentation file should include compact LOCS metadata where practical.

Use this default convention for source files:

```yaml
LOCS:
  capability: <short-capability-name>
  stability: draft
  owner: core
  kind: <cli|config|policy|parser|patch|verify|audit|report|test>
guard:
  mode: editable
```

Use comment syntax appropriate to the language.

Example for TypeScript/Rust-style block comment:

```txt
/*
LOCS:
  capability: policy-resolution
  stability: draft
  owner: core
  kind: policy
guard:
  mode: editable
*/
```

Example for Markdown frontmatter:

```yaml
---
locs:
  document_type: design
  stability: draft
  owner: core
guard:
  mode: review_required
---
```

Rules:

1. New files should default to `stability: draft`.
2. New files should default to `guard.mode: editable` unless they contain policy, architecture, audit, or security-critical content.
3. Do not mark new files as `protected` or `frozen` unless explicitly required.
4. Preserve existing LOCS metadata unless the task explicitly requires changing it.
5. Treat removal or weakening of LOCS as a policy-sensitive change.
6. Keep LOCS compact. Do not bloat headers with long explanations.

---

## 5. Architecture to Build

Build the following architecture:

```txt
User / LLM / Agent
      ↓
Patch Proposal
      ↓
Patch Normaliser
      ↓
Project Parser Layer
      ↓
Policy Resolver
      ↓
Deterministic Verifier
      ↓
Evidence Checker
      ↓
Decision Engine
      ↓
Apply / Reject / Require Review / Request Unlock
```

Core modules:

| Module | Purpose |
|---|---|
| CLI Adapter | Parse commands and invoke core services. |
| Config Loader | Load and validate `.guardpatch.yml`. |
| Parser Registry | Select parser by file type. |
| Markdown Parser | Build document sections and ranges. |
| Code Parser | Later AST support for functions/classes/signatures. |
| LOCS Extractor | Read LOCS metadata from comments/frontmatter. |
| Policy Resolver | Merge all policy sources deterministically. |
| Patch Normaliser | Convert diffs/structured patches into internal operations. |
| Verifier | Enforce edit rules. |
| Evidence Checker | Run tests/typecheck/lint where configured. |
| Audit Store | Record verification decisions and lifecycle events. |
| Reporter | Produce human-readable and JSON reports. |

---

## 6. Non-Negotiable Behaviour

GuardPatch must obey these rules:

1. Never trust the LLM's description of a patch.
2. Always compute actual changes from before/after content or parsed diff.
3. Never write to disk during verification.
4. Apply patches only after verification allows it.
5. Rejected patches must leave the workspace unchanged.
6. Deleting guard markers is a protected edit.
7. Removing or weakening LOCS metadata is policy-sensitive.
8. Policy files are protected by default if configured.
9. Reports must explain what was rejected and why.
10. Standalone functionality must not require Semantic CLI.

---

## 7. Phase 1 Implementation Instructions

Implement deterministic boundaries first.

Required capabilities:

- CLI scaffold
- `.guardpatch.yml` loader
- file/path rules
- lock first N lines
- exact locked ranges
- marker regions
- Markdown heading section protection
- frontmatter protection
- code block protection
- unified diff parsing
- in-memory patch application
- verification reports
- JSON output
- audit log
- guarded apply command

The Phase 1 CLI commands must include:

```bash
guardpatch init
guardpatch scan
guardpatch status
guardpatch verify <patch>
guardpatch apply <patch>
guardpatch explain <patch>
guardpatch audit
```

Success criteria:

- GuardPatch can reject a patch that modifies a locked Markdown section.
- GuardPatch can reject a patch that modifies the first N locked lines of a file.
- GuardPatch can reject deletion of guard markers.
- GuardPatch can allow edits inside declared editable regions.
- GuardPatch can explain each decision clearly.

---

## 8. Phase 2 Implementation Instructions

Implement LOCS-aware governance.

Required capabilities:

- define LOCS schema subset
- extract LOCS from Markdown frontmatter
- extract LOCS from code comments
- merge LOCS guard metadata into policy resolution
- detect LOCS weakening
- enforce LOCS requirement for new files where configured
- provide LOCS templates

Important:

LOCS describes project meaning and intended governance. GuardPatch still enforces using deterministic policy resolution.

Success criteria:

- A Markdown file with frontmatter guard mode can be protected.
- A source file with LOCS guard metadata can require review.
- A patch weakening `stable` to `draft` is detected.
- A new non-trivial source file without LOCS is flagged when required.

---

## 9. Phase 3 Implementation Instructions

Implement AST-aware code governance.

Required capabilities:

- parser registry
- TypeScript/JavaScript AST adapter
- Python AST adapter
- symbol registry
- function protection
- class protection
- method protection
- signature locking
- exported API locking
- import/dependency drift checks
- test weakening heuristics
- structured patch format

Success criteria:

- GuardPatch can reject edits to a protected function.
- GuardPatch can allow a body edit while rejecting a signature change.
- GuardPatch can detect exported API removal.
- GuardPatch can flag suspicious test weakening.
- GuardPatch can process structured JSON patches.

---

## 10. Phase 4 Implementation Instructions

Implement evidence-gated stability lifecycle.

Required capabilities:

- lifecycle states: `draft`, `active`, `stabilising`, `stable`, `protected`, `frozen`
- Git history checks
- test/typecheck/lint command runner
- `guardpatch promote`
- unlock registry
- `guardpatch unlock`
- unlock scopes
- automatic relock
- audit reporting

Success criteria:

- GuardPatch can promote a target to protected only after evidence passes.
- GuardPatch can reject promotion when tests fail.
- GuardPatch can allow one-patch unlock with reason.
- GuardPatch consumes one-patch unlock after use.

---

## 11. Phase 5 Implementation Instructions

Implement agent-aware edit authority.

Required capabilities:

- actor model
- agent profiles
- per-agent allow/deny rules
- proposal-only mode
- review queue
- deterministic risk scoring
- JSON adapter mode for LLM coding tools
- optional MCP/tool adapter only after CLI is stable

Success criteria:

- A frontend agent can be blocked from editing auth files.
- A documentation agent can be blocked from protected architecture sections.
- Proposal-only edits are queued instead of applied.
- Risk score appears in verification reports.
- External tools can call GuardPatch with stable JSON I/O.

---

## 12. Later Semantic CLI Integration

Do not make Semantic CLI a dependency for standalone GuardPatch.

After Phase 1–5, prepare optional integration:

- import Semantic AST graph
- import symbol registry
- import dependency graph
- import safe edit windows
- import risk tags
- request impact reports
- expose Semantic-facing commands

Possible commands:

```bash
semantic guard scan
semantic guard verify patch.diff
semantic guard explain patch.diff
semantic guard suggest-policy
semantic guard impact <target>
```

Semantic CLI should enhance GuardPatch by improving project understanding. GuardPatch remains the enforcement layer.

---

## 13. Policy Resolution Rules

Resolve policy deterministically in this order:

1. Emergency deny rules
2. Explicit user command override
3. Inline LOCS/guard metadata
4. `.guardpatch.yml`
5. Directory policy
6. Default project policy

When there is conflict, choose the safer/more restrictive rule unless an explicit authorised unlock exists.

Examples:

- `protected` overrides `editable`.
- `human_only` overrides agent permissions.
- `frozen` requires highest ceremony.
- `append_only` allows addition but not modification/deletion.
- deleting a policy source requires review or rejection.

---

## 14. Patch Decision Types

Use these statuses:

```txt
allowed
rejected
review_required
unlock_required
proposal_only
error
```

Every decision report must include:

- status
- summary
- changed files
- changed nodes
- violated policies
- policy sources
- evidence results if any
- suggested next action

---

## 15. Testing Requirements

Use fixture-based tests.

Each fixture should contain:

```txt
before/
patch.diff or patch.json
.guardpatch.yml
expected.json
```

Test categories:

- allowed patch
- locked line violation
- locked marker violation
- locked Markdown section violation
- LOCS weakening
- missing LOCS in new file
- protected function edit
- signature lock violation
- exported API change
- test weakening
- unlock workflow
- agent authority violation

Adversarial tests are mandatory.

Test attempts should include:

- deleting guard markers
- changing locked content while keeping headings
- weakening LOCS metadata
- editing policy file to self-authorise
- hiding a protected change inside broad formatting
- skipping tests instead of fixing code

---

## 16. Implementation Style

Prioritise:

1. deterministic behaviour
2. clear reports
3. small cohesive modules
4. fixture coverage
5. plain text config
6. minimal token overhead in LOCS
7. standalone usefulness
8. later Semantic compatibility

Avoid:

- relying on LLM reasoning for enforcement
- hidden state that cannot be audited
- premature Semantic coupling
- overcomplicated schemas in Phase 1
- automatic locking without evidence
- direct file writes before verification

---

## 17. Suggested First Milestone

The first working milestone should demonstrate this workflow:

```bash
guardpatch init
guardpatch verify fixtures/phase1_locked_markdown_section/patch.diff
guardpatch explain fixtures/phase1_locked_markdown_section/patch.diff
guardpatch apply fixtures/phase1_allowed_edit/patch.diff
```

Expected result:

- protected Markdown section edit is rejected
- allowed editable section edit is applied
- audit log records both events
- report is readable by a developer and an LLM agent

---

## 18. Final Instruction

Build GuardPatch as a serious safety layer for AI-assisted work.

Do not treat it as a Markdown locking toy.

The goal is to create:

> A deterministic, AST-aware, document-aware, version-aware edit authority layer that allows LLM coding tools to operate safely inside user-defined and evidence-backed boundaries.
