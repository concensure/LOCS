# LOCS - LLM-Optimised Capability Specification (v2.0)

**LOCS** is a portable framework for writing code that LLMs can reliably read, retrieve, compose, and govern. It transforms a standard codebase into a machine-readable capability marketplace.

---

## The Problem

Modern LLM workflows waste tokens in three places:

1. blind file loads
2. ambiguous file/module naming
3. weak governance around what metadata claims versus what code actually does

LOCS addresses that by turning source files into self-describing capability units with registry-level routing.

---

## Benefits

- **Fewer wasted tokens** — LLMs load only the capabilities they need via registry routing instead of blindly reading entire files.
- **Predictable retrieval** — self-describing metadata and strict `@key: value` syntax give LLMs a consistent surface to query, reducing hallucinated file paths and wrong module guesses.
- **Governance by default** — AST-backed validation catches mismatches between what metadata claims and what code actually does before they reach a model context.
- **Auditable token budgets** — per-file `@token-metrics` with a recorded backend make cost comparisons reproducible across models and providers.
- **Composable capability graph** — dependency declarations and circular-dependency detection let LLMs reason about safe composition order without reading implementation bodies.
- **Cross-project reuse** — the optional `LOCS_GRAND_REGISTRY.md` lets shared capabilities be discovered and routed across repos without duplicating code.
- **Model-agnostic** — token counting backends (tiktoken, transformers, sentencepiece, heuristic) mean the same framework works whether you are targeting GPT-4o, Qwen, Gemini, or a local model.

---

## Stability Lifecycle

Modules progress through a linear stability chain:

```
draft → active → stabilising → stable → protected → frozen
```

| Level | Meaning |
|---|---|
| `draft` | Work in progress, no guarantees |
| `active` | In use, may still change |
| `stabilising` | Hardening toward stable; breaking changes need review |
| `stable` | Production-ready, breaking changes require evidence |
| `protected` | Frozen API surface; edits require evidence + approval |
| `frozen` | Immutable; no LLM edits permitted without an explicit unlock |

`protected` and `frozen` map directly to GuardPatch enforcement — files at these levels cannot be edited by an AI agent without an explicit `guardpatch unlock`.

---

## Registry Model

`LOCS_REGISTRY.md` is the default project-local registry.

`LOCS_GRAND_REGISTRY.md` is optional and exists for cross-project sharing. Use it only when shared capability reuse is worth the larger search surface.

A `.locs.index.json` sidecar is maintained automatically by `locs register` for fast O(1) category and domain lookups. Run `locs index rebuild` to regenerate it from the registry table.

---

## What LOCS Enforces

`locs validate` checks:

- strict `@key: value` syntax
- required metadata fields
- capability-score presence
- section ordering
- dependency existence
- circular dependency detection
- dependency-depth validation
- Python AST-based signature and side-effect checks (exact)
- JavaScript/TypeScript AST-based checks when Tree-sitter extras are installed (exact)

Validation output includes a confidence report showing which AST and token backends were used:

```
PASS  my_module.py  (grade B)
  AST backend:   python-ast (exact)
  Token backend: tiktoken:cl100k_base (exact)
  Coverage:      5/5 declared inputs verified by AST
```

If Tree-sitter is unavailable, the report marks the backend as `regex-fallback (degraded)` so you always know your validation confidence level.

---

## Token Counting Backends

Token counts are backend-specific.

- `tiktoken` for OpenAI-family usage
- `transformers` tokenizers for Hugging Face models
- `sentencepiece` for SentencePiece model files
- heuristic fallback when no exact backend is available

LOCS records the backend in `@token-metrics` so counts remain auditable. Only compare token counts from the same backend family.

---

## Supported Languages

LOCS metadata headers work in: TypeScript, JavaScript, Python, Go, Rust, Java, C, C++, Ruby, Shell, Lua, PHP.

For Ruby: use `=begin` / `=end` block comments.
For Shell: use `# BEGIN_LOCS` / `# END_LOCS` markers.
For languages not listed: the default C-style `/* ... */` block is used as fallback.

---

## Quick Start

Install the framework and initialise LOCS governance in your project with a single sequence:

```bash
pip install locs-cli && locs init
```

`locs init` scans your project, asks three short GuardPatch questions (with recommendations based on what it finds), then writes:

- `LOCS_SKILL.md` — generation rules for LLM sessions
- `LOCS_SESSION_INIT.md` — session bootstrap workflow
- `.guardpatch.yml` — file-protection policy tuned to your project
- `LOCS_REGISTRY.md` — local capability registry
- `.git/hooks/pre-commit` — validates LOCS modules before every commit
- `CLAUDE.md` — patched with a LOCS section so Claude picks up the rules automatically

**Flags:**

| Flag | Effect |
|------|--------|
| `--yes` / `-y` | Accept all recommended defaults, no prompts |
| `--dry-run` | Preview what would be written, write nothing |
| `--force` | Overwrite existing `.guardpatch.yml` and doc files |
| `--no-hook` | Skip pre-commit hook installation |
| `--no-claude-md` | Skip `CLAUDE.md` creation/patch |

To install into a different directory:

```bash
locs init /path/to/project
```

---

## Installation

Base install:

```bash
pip install locs-cli
```

Editable (development) install from source:

```bash
pip install -e .
```

Optional extras:

```bash
pip install locs-cli[openai]
pip install locs-cli[huggingface]
pip install locs-cli[google]
pip install locs-cli[js_ast]
pip install locs-cli[full]
```

---

## Usage

```bash
# Initialise governance in a project (interactive)
locs init

# Accept all recommended defaults without prompting
locs init --yes

# Preview what would be written
locs init --dry-run

# Scaffold a new module
locs new graph.smart-port-selector --ext .py

# Score with automatic backend selection
locs score graph_smart_port_selector.py --write

# Force OpenAI counting
locs score graph_smart_port_selector.py --write --tokenizer tiktoken --model gpt-4o-mini

# Force Hugging Face counting
locs score graph_smart_port_selector.py --write --tokenizer transformers --model Qwen/Qwen2.5-Coder-7B

# Force SentencePiece counting from a local model file
locs score graph_smart_port_selector.py --write --tokenizer sentencepiece --tokenizer-resource .\gemini.model

# Validate (prints AST and token backend confidence)
locs validate graph_smart_port_selector.py

# Register locally (also updates .locs.index.json)
locs register graph_smart_port_selector.py

# Optional shared publication
locs register graph_smart_port_selector.py --scope shared

# Bootstrap compact routing context (uses index when available)
locs bootstrap --category graph --limit 5

# Registry index management
locs index status
locs index rebuild
```

---

## File Reference

- `locs.py`: CLI engine
- `LOCS_REGISTRY.md`: default local registry
- `.locs.index.json`: fast-lookup index (auto-maintained by `locs register`)
- `LOCS_GRAND_REGISTRY.md`: optional shared registry
- `LOCS_SKILL.md`: generation rules
- `LOCS_SESSION_INIT.md`: LLM bootstrap file
- `LOCS_CAPABILITY_SCORING.md`: scoring model
- `docs/architecture.md`: framework design

---

## Notes

- Token counts should only be compared directly when they come from the same backend family.
- `locs validate` always reports which AST and token backends were used — check the confidence line.
- Python AST analysis uses the built-in `ast` module (always exact).
- JavaScript/TypeScript AST analysis uses optional Tree-sitter backends when installed; falls back to regex otherwise.
- Stability levels map directly to GuardPatch guard modes: `protected` → `mode: protected`, `frozen` → `mode: frozen`.
