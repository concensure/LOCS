# LOCS Capability Scoring Algorithm (v1.4)

Defines the deterministic scoring model for `@capability-score`.

---

## Score Overview

```
C = 0.25R + 0.20D + 0.20T + 0.15P + 0.15Q + 0.05I
```

| Symbol | Meaning |
|---|---|
| `R` | Retrieval clarity |
| `D` | Determinism and safety |
| `T` | Token efficiency |
| `P` | Performance |
| `Q` | Structural quality |
| `I` | Isolation and usage |

---

## Token Efficiency

`T` is derived from `@token-metrics`.

LOCS supports:

- `tiktoken`
- `transformers`
- `sentencepiece`
- heuristic fallback

The backend used should be recorded in metadata. Cross-backend comparisons should be treated as approximate, not absolute.

---

## Determinism and Safety

This dimension remains separate from performance.

Side-effect honesty is increasingly important because LOCS now supports AST-based validation in Python and optional Tree-sitter checks for JavaScript/TypeScript.

---

## Isolation and Usage

Popularity remains bounded. Usage metrics can improve discoverability but must not dominate code quality or determinism.
