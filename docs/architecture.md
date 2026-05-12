# LOCS Framework Architecture

## Overview

LOCS is a retrieval and governance framework for turning source files into machine-readable capability units. Its goal is lower token usage, higher routing precision, and safer autonomous composition.

## Core Components

### 1. Metadata Schema

Each LOCS module starts with a flat, deterministic metadata header.

- strict `@key: value` syntax
- shallow capability taxonomy
- compact retrieval signal
- truth-in-advertising contract with the implementation

### 2. Registry Layer

LOCS supports two registry scopes:

- `LOCS_REGISTRY.md`: local default
- `LOCS_GRAND_REGISTRY.md`: optional shared scope

The local registry remains the preferred default because it keeps retrieval narrow and cheap.

### 3. Token Metrics Layer

Token counting is backend-aware rather than universal.

- OpenAI-family: `tiktoken`
- Hugging Face-family: `transformers`
- SentencePiece-family: `sentencepiece`
- fallback: heuristic

Counts should be compared within the same tokenizer family, not across unrelated backends.

### 4. Validation Layer

Validation uses a tiered analysis model:

- Python: built-in `ast`
- JavaScript/TypeScript: optional Tree-sitter parsing
- fallback: deterministic regex heuristics

This gives LOCS stronger signature and side-effect checks without forcing heavyweight dependencies on every install.

## Enforcement Model

LOCS separates enforcement into four layers. Each layer has a distinct scope and blocking behaviour to avoid friction in AI-assisted coding workflows.

### Layer 1 — Generation (session context)

`LOCS_SKILL.md` and `CLAUDE.md` are loaded by the AI at session start. The AI follows the generation rules natively, producing correctly-annotated modules before any file is written to disk. This is the cheapest and most effective layer.

### Layer 2 — Pre-stage (developer-enforced)

`locs validate <file>` is the primary validation gate. It must be run after scoring and before `git add`. Validation failures must be resolved before staging. This is developer-enforced via workflow instructions, not automated.

### Layer 3 — Pre-commit (repo-local hook)

`.git/hooks/pre-commit` validates every staged file that already carries a `@locs-version` header. It uses a fast `grep` scan to identify LOCS files — no LLM calls, no AST parsing at commit time. It only blocks commits for files that already opted into the framework but failed validation. Use `locs hook install` to scope it to the current repo only (never global).

### Layer 4 — Advisory and CI

`locs audit` scans a directory for files that look like capabilities but lack `@locs-version` headers. It uses lightweight regex heuristics (exported symbols + LOC count). It never blocks a commit. Designed for:

- On-demand discovery: `locs audit`
- CI PR checks: `locs audit --exit-nonzero` (exits 1 if ungoverned files are found)
- Machine-readable output: `locs audit --format json`

### What does NOT happen

- No LLM calls in any automated step (pre-commit, CI audit).
- No global git template installation. Hook scope is always repo-local.
- No complexity-based commit blocking. The pre-commit hook does not apply heuristics to unannotated files.
- No persistent state beyond the registry and index files. No sidecar databases.

---

## Governance Model

Hard checks:

- required metadata presence
- strict metadata syntax
- capability-score presence
- section ordering
- module-id format
- dependency existence
- circular dependency detection
- dependency-depth cap and consistency

AST-grade checks where supported:

- declared inputs match parsed function signatures
- declared output type is present in parsed signatures when expressible
- side-effect classification is checked against parsed call sites

Fallback checks:

- regex-based signature detection
- regex-based side-effect heuristics

## Shared Registry Positioning

The shared registry is opt-in. It is for:

- workspace-level capability marketplaces
- cross-project capability reuse
- teams that want shared module discovery

It should not replace the local registry as the default development path.
