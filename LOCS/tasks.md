# GuardPatch Tasks

This task plan develops GuardPatch as a standalone CLI from Phase 1 to Phase 5, then prepares later integration with Semantic CLI. Tasks are dependency-ordered. Do not skip earlier foundations even if later features appear more valuable.

---

## Phase 0 — Repository and Engineering Foundation

### 0.1 Create repository scaffold

**Depends on:** none  
**Output:** initial project structure

Create:

```txt
guardpatch/
  README.md
  DESIGN.md
  tasks.md
  prompts/
  fixtures/
  src/
  tests/
  .guardpatch.yml
```

If using Rust:

```txt
src/
  main.rs
  cli.rs
  config/
  policy/
  parser/
  patch/
  verify/
  audit/
  report/
```

If using TypeScript:

```txt
src/
  cli.ts
  config/
  policy/
  parser/
  patch/
  verify/
  audit/
  report/
```

Acceptance criteria:

- project builds
- basic CLI command runs
- CI placeholder exists
- README explains GuardPatch in one paragraph

---

### 0.2 Add LOCS header convention for new source files

**Depends on:** 0.1  
**Output:** LOCS file header template

Every new non-trivial source file should include compact LOCS metadata where practical.

Example:

```yaml
LOCS:
  capability: <short-capability-name>
  stability: draft
  owner: core
  kind: <cli|parser|policy|verifier|report|test>
guard:
  mode: editable
```

Acceptance criteria:

- documented LOCS convention exists
- initial source files use LOCS headers or documented equivalent comments
- no file marks itself protected unless explicitly required

---

### 0.3 Establish fixture testing framework

**Depends on:** 0.1  
**Output:** test harness for fixture workspaces

Create fixture format:

```txt
fixtures/
  phase1_locked_markdown_section/
    before/
    patch.diff
    .guardpatch.yml
    expected.json
```

Acceptance criteria:

- test harness can load fixture
- test harness can compare actual decision to expected decision

---

## Phase 1 — Deterministic Boundaries

Goal: prevent accidental edits to obvious protected surfaces.

---

### 1.1 Implement CLI command structure

**Depends on:** 0.1  
**Output:** CLI command skeleton

Commands:

```bash
guardpatch init
guardpatch scan
guardpatch status
guardpatch verify <patch>
guardpatch apply <patch>
guardpatch explain <patch>
guardpatch audit
```

Acceptance criteria:

- each command returns a stable placeholder response
- invalid command usage gives helpful error

---

### 1.2 Implement config loader

**Depends on:** 1.1  
**Output:** `.guardpatch.yml` parser

Support:

- project default mode
- path rules
- line rules
- document rules
- patch limits
- unlock settings placeholder

Acceptance criteria:

- valid YAML loads into typed config
- invalid YAML gives clear error
- missing config uses safe defaults

---

### 1.3 Implement glob/path rule matching

**Depends on:** 1.2  
**Output:** file-level policy resolver

Support:

- exact path match
- glob match
- default mode
- protected policy files

Acceptance criteria:

- path rules resolve correctly
- more specific path wins over broad default
- policy files are protected by default when configured

---

### 1.4 Implement unified diff parser

**Depends on:** 1.1  
**Output:** patch normaliser for unified diffs

Support:

- changed files
- added files
- deleted files
- hunks
- changed line ranges

Acceptance criteria:

- parser handles standard git diff
- parser reports malformed diff clearly
- parser extracts changed files and line ranges

---

### 1.5 Implement in-memory patch application

**Depends on:** 1.4  
**Output:** candidate file state without writing to disk

Acceptance criteria:

- applies valid diff to in-memory file map
- rejects patch if context lines do not match
- does not modify working files during verification

---

### 1.6 Implement line-range verifier

**Depends on:** 1.2, 1.4, 1.5  
**Output:** lock-first-lines and locked-range checks

Support:

- `lock_first_lines`
- exact locked ranges
- file deletion check

Acceptance criteria:

- patch changing locked first lines is rejected
- patch changing unlocked lines is allowed
- file deletion is rejected when protected

---

### 1.7 Implement marker parser

**Depends on:** 1.5  
**Output:** marker region detection

Markers:

```txt
GUARD:LOCKED id=<id>
/GUARD:LOCKED
GUARD:EDITABLE id=<id>
/GUARD:EDITABLE
GUARD:PROPOSAL_ONLY id=<id>
/GUARD:PROPOSAL_ONLY
```

Support comment styles for:

- Markdown/HTML comments
- TypeScript/JavaScript line comments
- Python comments

Acceptance criteria:

- detects marker ranges
- rejects unbalanced markers
- rejects deletion of guard markers

---

### 1.8 Implement marker-region verifier

**Depends on:** 1.7  
**Output:** locked/editable marker enforcement

Acceptance criteria:

- changing locked marker region is rejected
- changing editable marker region is allowed
- deleting marker boundary is rejected
- changing proposal-only region returns review/proposal status

---

### 1.9 Implement Markdown section parser

**Depends on:** 1.5  
**Output:** heading tree with line ranges

Support:

- ATX headings `#`, `##`, etc.
- section subtree ranges
- frontmatter range
- fenced code block ranges

Acceptance criteria:

- parser maps heading titles to ranges
- nested sections are handled correctly
- code blocks do not create false headings

---

### 1.10 Implement Markdown section verifier

**Depends on:** 1.2, 1.9  
**Output:** document AST policy enforcement

Support:

- `lock_frontmatter`
- `lock_sections`
- `editable_sections`
- `lock_code_blocks`

Acceptance criteria:

- locked section edits are rejected
- editable section edits are allowed
- frontmatter edits are rejected when locked
- code block edits are rejected when locked

---

### 1.11 Implement basic report generator

**Depends on:** 1.6, 1.8, 1.10  
**Output:** human-readable verification report

Report must include:

- decision status
- changed files
- violated policies
- target nodes
- suggested next action

Acceptance criteria:

- rejection report explains exact reason
- allowed report lists checked policies
- output is suitable for LLM coding tools and humans

---

### 1.12 Implement JSON decision output

**Depends on:** 1.11  
**Output:** machine-readable decision report

Acceptance criteria:

- `--json` flag returns stable schema
- includes status, reasons, files, nodes, policy sources

---

### 1.13 Implement audit log

**Depends on:** 1.11, 1.12  
**Output:** JSONL audit trail

Audit events:

- verify patch
- apply patch
- reject patch
- policy load error

Acceptance criteria:

- audit file records event per verification
- audit record includes timestamp, status, reasons
- audit command displays recent events

---

### 1.14 Implement apply command with verification gate

**Depends on:** 1.5, 1.11, 1.13  
**Output:** guarded patch application

Acceptance criteria:

- allowed patch can be applied
- rejected patch is not applied
- review-required patch is not applied without explicit approval flag

---

### 1.15 Phase 1 fixture suite

**Depends on:** 1.6, 1.8, 1.10, 1.14  
**Output:** regression fixtures

Create fixtures for:

- locked first 10 lines
- locked Markdown section
- editable Markdown section
- locked marker block
- marker deletion
- protected file deletion
- patch allowed case

Acceptance criteria:

- all fixtures pass
- Phase 1 success metrics documented in README

---

## Phase 2 — LOCS-Aware Governance

Goal: use LOCS metadata to define project intent and guard policy.

---

### 2.1 Define LOCS metadata schema

**Depends on:** 0.2, 1.2  
**Output:** formal LOCS schema subset for GuardPatch

Fields:

```yaml
locs:
  capability: string
  stability: draft|active|stabilising|stable|protected|frozen
  owner: string
  kind: string
guard:
  mode: editable|proposal_only|review_required|protected|frozen|append_only|generated|human_only|deprecated
  lock_signature: boolean
  lock_body: boolean
  require_tests: boolean
  unlock_requires: string
```

Acceptance criteria:

- schema documented
- invalid LOCS metadata produces warning or error according to config

---

### 2.2 Implement Markdown frontmatter LOCS extractor

**Depends on:** 1.9, 2.1  
**Output:** LOCS extraction from Markdown/YAML frontmatter

Acceptance criteria:

- extracts `locs` and `guard` blocks
- maps document-level guard mode into policy resolver
- rejects or warns on invalid values

---

### 2.3 Implement code comment LOCS extractor

**Depends on:** 2.1  
**Output:** LOCS extraction from leading comments

Support initially:

- TypeScript/JavaScript block comments
- Python leading comments/docstring fallback

Acceptance criteria:

- extracts LOCS from file header
- extracts LOCS from symbol-adjacent comments where possible
- invalid metadata reports clear diagnostics

---

### 2.4 Integrate LOCS into policy resolver

**Depends on:** 2.2, 2.3, 1.3  
**Output:** LOCS-derived policy rules

Priority order:

1. emergency deny rules
2. explicit command override
3. inline LOCS/guard metadata
4. `.guardpatch.yml`
5. default policy

Acceptance criteria:

- LOCS guard mode affects verification
- policy source is shown in reports
- conflict resolution is deterministic

---

### 2.5 Implement LOCS weakening detection

**Depends on:** 2.4, 1.5  
**Output:** detect suspicious metadata downgrades

Examples:

- `stable` to `draft`
- `protected` to `editable`
- removing invariants
- removing public API marker
- removing guard block

Acceptance criteria:

- weakening returns review_required or rejected according to config
- report clearly explains metadata weakening

---

### 2.6 Implement new-file LOCS requirement

**Depends on:** 2.2, 2.3, 1.4  
**Output:** check created files for LOCS metadata

Config:

```yaml
project:
  locs_required_for_new_files: true
```

Acceptance criteria:

- new source/doc files without LOCS return review_required or rejected
- generated files can be exempted
- tiny trivial files can be exempted by config

---

### 2.7 Add LOCS templates

**Depends on:** 2.1  
**Output:** templates for common file types

Templates:

- source file
- CLI command file
- parser adapter
- policy module
- verifier module
- Markdown design doc
- test fixture

Acceptance criteria:

- CLI can print template
- prompt document references templates

---

### 2.8 Phase 2 fixture suite

**Depends on:** 2.4, 2.5, 2.6  
**Output:** LOCS regression tests

Fixtures:

- LOCS protected Markdown doc
- LOCS review-required source file
- LOCS weakening patch
- new file missing LOCS
- new file with valid LOCS

Acceptance criteria:

- all fixtures pass

---

## Phase 3 — AST-Aware Code Governance

Goal: protect functions, classes, signatures, exports and public API.

---

### 3.1 Add parser registry

**Depends on:** 1.2, 2.4  
**Output:** file extension to parser adapter mapping

Acceptance criteria:

- parser adapters can be registered
- unsupported files fall back to text/line/marker policies

---

### 3.2 Implement TypeScript/JavaScript AST adapter

**Depends on:** 3.1  
**Output:** AST nodes for JS/TS files

Extract:

- functions
- arrow functions assigned to const
- classes
- methods
- interfaces/types
- enums
- imports
- exports

Acceptance criteria:

- fixture files produce stable node IDs
- node ranges are accurate
- exported symbols are detected

---

### 3.3 Implement Python AST adapter

**Depends on:** 3.1  
**Output:** AST nodes for Python files

Extract:

- functions
- classes
- methods
- imports
- module-level constants

Acceptance criteria:

- fixture files produce stable node IDs
- node ranges are accurate

---

### 3.4 Implement symbol registry

**Depends on:** 3.2, 3.3  
**Output:** map symbol names to governable nodes

Acceptance criteria:

- symbol lookup by fully qualified target works
- duplicate symbol names are disambiguated by file path
- registry can be serialised for scan output

---

### 3.5 Implement function/class protection

**Depends on:** 3.4, 1.5  
**Output:** AST node change detection

Acceptance criteria:

- changing protected function is rejected
- changing protected class is rejected
- changing unprotected symbol is allowed
- reports identify symbol name and file path

---

### 3.6 Implement signature locking

**Depends on:** 3.5  
**Output:** allow body changes while preserving signature

Acceptance criteria:

- body-only change allowed when configured
- parameter/return/export changes rejected when signature locked
- report identifies signature violation

---

### 3.7 Implement exported API locking

**Depends on:** 3.4, 3.6  
**Output:** public API snapshot comparison

Acceptance criteria:

- export removal is rejected
- exported signature change is rejected when locked
- new export returns review_required if configured

---

### 3.8 Implement import/dependency drift checks

**Depends on:** 3.2, 3.3  
**Output:** import and dependency change detection

Acceptance criteria:

- new import in protected file returns review_required
- dependency/package file change requires approval when configured

---

### 3.9 Implement test weakening heuristics

**Depends on:** 3.2, 3.3  
**Output:** detect suspicious test changes

Heuristics:

- removed assertion
- skipped test
- relaxed expected value
- deleted test file
- changed test to match implementation without source change reason

Acceptance criteria:

- common weakening cases are review_required or rejected
- false positives are explainable and overrideable

---

### 3.10 Implement structured patch format

**Depends on:** 1.5, 3.4  
**Output:** JSON patch operations

Operations:

- `replace_file`
- `replace_range`
- `replace_section`
- `append_section`
- `replace_symbol_body`
- `replace_symbol_signature`
- `insert_after_marker`
- `delete_file`
- `rename_file`
- `create_file`

Acceptance criteria:

- structured patch validates against schema
- operations convert to internal edit operations
- invalid target is reported clearly

---

### 3.11 Phase 3 fixture suite

**Depends on:** 3.5, 3.6, 3.7, 3.8, 3.9, 3.10  
**Output:** AST governance regression tests

Fixtures:

- protected function body change
- signature locked body allowed
- signature change rejected
- exported API removal
- protected class edit
- test weakening
- structured patch replacing symbol body

Acceptance criteria:

- all fixtures pass

---

## Phase 4 — Evidence-Gated Stability Lifecycle

Goal: promote content to stable/protected based on evidence and support safe unlock workflows.

---

### 4.1 Implement promotion state model

**Depends on:** 2.1, 3.4  
**Output:** lifecycle states for nodes

States:

```txt
draft → active → stabilising → stable → protected → frozen
```

Acceptance criteria:

- state can be read from LOCS/config
- state can be reported by scan/status

---

### 4.2 Implement Git history checks

**Depends on:** 4.1  
**Output:** no-changes-for-commits evidence

Acceptance criteria:

- can detect last changed commit for file/node where possible
- supports `no_changes_for_commits: N`
- degrades gracefully outside git repo

---

### 4.3 Implement evidence command runner

**Depends on:** 1.2  
**Output:** run test/typecheck/lint commands from config

Example:

```yaml
evidence:
  test: "npm test"
  typecheck: "npm run typecheck"
  lint: "npm run lint"
```

Acceptance criteria:

- commands run only when required/configured
- output is captured in report
- failures block promotion or application according to policy

---

### 4.4 Implement promotion command

**Depends on:** 4.1, 4.2, 4.3  
**Output:** `guardpatch promote`

Acceptance criteria:

- promotion checks configured evidence
- failed evidence blocks promotion
- successful promotion updates policy/registry according to configured storage mode
- audit event recorded

---

### 4.5 Implement unlock registry

**Depends on:** 1.13, 4.1  
**Output:** store unlock grants

Unlock fields:

- target
- reason
- actor
- scope
- expiry
- created_at
- policy source

Acceptance criteria:

- unlock can be created
- unlock can be listed
- expired unlock is ignored

---

### 4.6 Implement unlock command

**Depends on:** 4.5  
**Output:** `guardpatch unlock`

Scopes:

- one_patch
- branch
- time_limited
- review_required
- migration

Acceptance criteria:

- reason is required when configured
- unlock applies only to target and scope
- audit event recorded

---

### 4.7 Integrate unlock into verifier

**Depends on:** 4.5, 4.6, 3.5  
**Output:** protected edit can proceed only when valid unlock exists

Acceptance criteria:

- protected edit without unlock is rejected
- protected edit with valid one-patch unlock proceeds or returns review_required according to policy
- one-patch unlock is consumed after use

---

### 4.8 Implement relock workflow

**Depends on:** 4.7  
**Output:** automatic or explicit relock after unlock scope ends

Acceptance criteria:

- one-patch unlock relocks automatically
- branch unlock reports when still active
- relock event is audited

---

### 4.9 Implement audit report

**Depends on:** 1.13, 4.4, 4.6  
**Output:** human-readable audit history

Acceptance criteria:

- can filter by target
- can filter by event type
- can show unlock/promotion history

---

### 4.10 Phase 4 fixture suite

**Depends on:** 4.4, 4.7, 4.8, 4.9  
**Output:** lifecycle regression tests

Fixtures:

- promote to protected with passing evidence
- promotion blocked by failing test command
- unlock one patch
- unlock expired
- relock after patch

Acceptance criteria:

- all fixtures pass

---

## Phase 5 — Agent-Aware Edit Authority

Goal: support multi-agent and autonomous coding workflows.

---

### 5.1 Define actor model

**Depends on:** 1.13  
**Output:** actor identity model

Actors:

- human
- llm_agent
- named_agent
- generator
- ci
- unknown

Acceptance criteria:

- actor can be passed via CLI flag/env var
- audit records actor identity

---

### 5.2 Implement agent profile config

**Depends on:** 5.1, 1.3  
**Output:** per-agent authority rules

Example:

```yaml
agents:
  frontend_agent:
    allow:
      - "src/ui/**"
      - "docs/ui/**"
    deny:
      - "src/auth/**"
      - ".guardpatch.yml"
```

Acceptance criteria:

- agent profile resolves allow/deny paths
- deny overrides allow
- report identifies agent authority violation

---

### 5.3 Implement proposal-only mode

**Depends on:** 5.2, 1.14  
**Output:** patch may be verified but not applied

Acceptance criteria:

- proposal-only target returns review/proposal status
- apply command refuses proposal-only patch without human approval

---

### 5.4 Implement review queue

**Depends on:** 5.3, 1.13  
**Output:** local queue of patches needing review

Acceptance criteria:

- review-required decisions can be saved
- queue can be listed
- queue item can be approved/rejected

---

### 5.5 Implement patch risk scoring

**Depends on:** 3.7, 3.8, 4.3, 5.2  
**Output:** low/medium/high/blocked risk rating

Signals:

- protected surface touched
- public API changed
- dependency changed
- test weakened
- patch size
- agent authority mismatch
- LOCS stability weakened

Acceptance criteria:

- risk score appears in report
- blocked status overrides score
- scoring is deterministic and documented

---

### 5.6 Add adapter mode for LLM coding tools

**Depends on:** 5.3, 5.5  
**Output:** CLI mode suitable for external tools

Features:

- JSON input/output
- stable exit codes
- concise machine-readable errors
- optional structured patch input

Acceptance criteria:

- LLM tool can call `guardpatch verify --json`
- exit codes distinguish allowed/rejected/review-required

---

### 5.7 Optional MCP/tool server adapter

**Depends on:** 5.6  
**Output:** adapter for agent tool use

Acceptance criteria:

- exposes verify/apply/explain/protect/unlock operations
- does not duplicate core logic
- CLI remains primary source of truth

---

### 5.8 Phase 5 fixture suite

**Depends on:** 5.2, 5.3, 5.4, 5.5, 5.6  
**Output:** agent governance tests

Fixtures:

- frontend agent blocked from auth file
- doc agent blocked from protected architecture section
- proposal-only patch queued for review
- high-risk patch scoring
- machine-readable adapter output

Acceptance criteria:

- all fixtures pass

---

## Phase 6 — Later Semantic CLI Integration

Goal: enhance GuardPatch with Semantic CLI's project understanding without making Semantic required.

---

### 6.1 Define Semantic integration contract

**Depends on:** Phase 3 complete  
**Output:** interface spec between GuardPatch and Semantic CLI

Contract should include:

- AST graph import
- symbol registry import
- dependency graph import
- safe edit window import
- risk tag import
- impact report request

Acceptance criteria:

- JSON schema for Semantic graph input exists
- GuardPatch can run without Semantic

---

### 6.2 Implement Semantic graph importer

**Depends on:** 6.1  
**Output:** optional graph enrichment

Acceptance criteria:

- imports Semantic symbol/dependency graph
- resolves nodes to GuardPatch governable nodes
- handles missing Semantic data gracefully

---

### 6.3 Integrate safe edit windows

**Depends on:** 6.2  
**Output:** verifier can use Semantic safe edit windows

Acceptance criteria:

- patch outside safe window is rejected/review-required
- patch inside safe window proceeds through normal policy checks

---

### 6.4 Integrate impact analysis

**Depends on:** 6.2  
**Output:** use Semantic impact report for risk scoring and review requirements

Acceptance criteria:

- high-impact changes require review
- impact report appears in GuardPatch explanation

---

### 6.5 Add Semantic CLI subcommands

**Depends on:** 6.3, 6.4  
**Output:** optional Semantic-facing commands

Commands:

```bash
semantic guard scan
semantic guard verify patch.diff
semantic guard explain patch.diff
semantic guard suggest-policy
semantic guard impact <target>
```

Acceptance criteria:

- commands delegate to GuardPatch core
- no duplicate policy logic

---

## Cross-Cutting Requirements

### Documentation requirements

Every phase must update:

- README
- DESIGN.md where architecture changes
- tasks.md progress markers
- example `.guardpatch.yml`
- fixture documentation

### LOCS requirements

When creating new files:

1. Add compact LOCS metadata unless file is trivial/generated.
2. Set `stability: draft` by default.
3. Set `guard.mode: editable` or `review_required` by default.
4. Do not mark new files protected unless user explicitly directs it.
5. Preserve existing LOCS metadata unless a task specifically requires changing it.

### Quality requirements

- deterministic tests must not rely on LLM output
- verifier must never trust patch self-description
- all writes must be gated by verification
- rejected patches must leave workspace unchanged
- reports must be understandable to both humans and LLM agents

---

## Completion Definition

GuardPatch reaches standalone production readiness when:

1. It can verify and apply patches safely.
2. It protects files, lines, markers, Markdown sections, LOCS metadata, and key code symbols.
3. It supports promotion/unlock lifecycle.
4. It supports agent profiles.
5. It provides clear reports and audit logs.
6. It has fixture coverage for normal, edge, and adversarial cases.
7. It can run independently of Semantic CLI.
8. It has a documented integration pathway for Semantic CLI.
