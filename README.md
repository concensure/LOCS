# LOCS - LLM-Optimised Capability Specification (v1.4)

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

## Registry Model

`LOCS_REGISTRY.md` is the default project-local registry.

`LOCS_GRAND_REGISTRY.md` is optional and exists for cross-project sharing. Use it only when shared capability reuse is worth the larger search surface.

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
- Python AST-based signature and side-effect checks
- JavaScript/TypeScript AST-based checks when Tree-sitter extras are installed

If AST-capable backends are unavailable, LOCS falls back to deterministic regex checks instead of failing hard.

---

## Token Counting Backends

Token counts are backend-specific.

- `tiktoken` for OpenAI-family usage
- `transformers` tokenizers for Hugging Face models
- `sentencepiece` for SentencePiece model files
- heuristic fallback when no exact backend is available

LOCS records the backend in `@token-metrics` so counts remain auditable.

---

## Installation

Base install:

```bash
pip install -e .
```

Optional extras:

```bash
pip install -e .[openai]
pip install -e .[huggingface]
pip install -e .[google]
pip install -e .[js_ast]
pip install -e .[full]
```

---

## Usage

```bash
# Scaffold
locs new graph.smart-port-selector --ext .py

# Score with automatic backend selection
locs score graph_smart_port_selector.py --write

# Force OpenAI counting
locs score graph_smart_port_selector.py --write --tokenizer tiktoken --model gpt-4o-mini

# Force Hugging Face counting
locs score graph_smart_port_selector.py --write --tokenizer transformers --model Qwen/Qwen2.5-Coder-7B

# Force SentencePiece counting from a local model file
locs score graph_smart_port_selector.py --write --tokenizer sentencepiece --tokenizer-resource .\gemini.model

# Validate
locs validate graph_smart_port_selector.py

# Register locally
locs register graph_smart_port_selector.py

# Optional shared publication
locs register graph_smart_port_selector.py --scope shared

# Bootstrap compact routing context
locs bootstrap --category graph --limit 5
```

---

## File Reference

- `locs.py`: CLI engine
- `LOCS_REGISTRY.md`: default local registry
- `LOCS_GRAND_REGISTRY.md`: optional shared registry
- `LOCS_SKILL.md`: generation rules
- `LOCS_SESSION_INIT.md`: LLM bootstrap file
- `LOCS_CAPABILITY_SCORING.md`: scoring model
- `docs/architecture.md`: framework design

---

## Notes

- Token counts should only be compared directly when they come from the same backend family.
- Python AST analysis uses the built-in `ast` module.
- JavaScript/TypeScript AST analysis uses optional Tree-sitter backends when installed.
