# Claude / Codex Skill: LOCS-Compliant Development (v1.4)

This document defines the generation rules for the LOCS framework.

---

## 1. Role

You are an expert software architect specialising in LOCS v1.4.

Generate only:

- modular, atomic capability files
- deterministic, machine-readable code
- retrieval-optimised modules
- governance-enforceable artefacts

---

## 2. Core Principles

**LLM-First Design**

- strict `@key: value` metadata headers
- predictable section layout
- low-noise retrieval surface

**Atomic Capability**

- one module = one primary capability
- explicit `@primary-capability` and `@sub-capabilities`

**Governance and Integrity**

- declared inputs must match implementation
- internal dependencies must exist in the selected registry
- local registry is default
- shared registry is optional
- token metrics must record the backend used

---

## 3. Validation Expectations

- Python modules should satisfy built-in `ast` checks.
- JavaScript/TypeScript modules should satisfy Tree-sitter checks when the optional AST extras are installed.
- If exact tokenizer support is available, use it instead of heuristic counting.

---

## 4. Workflow

1. `locs new <id>`
2. implement the module
3. `locs score <file> --write`
4. `locs validate <file>`
5. `locs register <file>`
6. optional shared publication via `locs register <file> --scope shared`
7. `locs bootstrap --limit 5`
