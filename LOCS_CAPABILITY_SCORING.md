# LOCS Capability Scoring Algorithm (v1.2)

Defines the deterministic mathematical model for computing `@capability-score` from LOCS metadata fields.

---

## 1. Score Overview

The capability score **C** is a normalised scalar in **[0, 1]**.

It is a weighted sum of six sub-scores, each derived from metadata fields and registry state.

```
C = w₁·R + w₂·D + w₃·T + w₄·P + w₅·Q + w₆·I
```

| symbol | sub-score | weight (w) | what it measures |
|---|---|---|---|
| R | Retrieval Clarity | 0.25 | how quickly an LLM can evaluate the module |
| D | Determinism & Safety | 0.20 | trust and predictability |
| T | Token Efficiency | 0.20 | density of metadata vs implementation |
| P | Performance | 0.15 | algorithmic efficiency via complexity |
| Q | Structural Quality | 0.15 | metadata completeness |
| I | Isolation & Usage | 0.05 | self-containment and ecosystem impact |

**Weights sum to 1.00.**

All sub-scores are in **[0, 1]** before weighting.

---

## 2. Sub-Score Definitions

### 2.1 Retrieval Clarity — R

Measures how quickly an LLM can decide to use or skip the module.

```
R = (primary_score + sub_cap_score + capability_score + name_score) / 4
```

- **primary_score**: 1.0 if `@primary-capability` is present, 0.0 otherwise.
- **sub_cap_score**: `min(n, 5) / 5` where `n` is count of `@sub-capabilities`.
- **capability_score**: 1.0 if length is 5-12 words, else scaled penalty.
- **name_score**: 1.0 if 2-4 PascalCase words, else scaled penalty.

---

### 2.2 Determinism & Safety — D

Measures trust and predictability of the module's behaviour.

```
D = (det_score + side_effect_score + state_score) / 3
```

- **det_score**: `deterministic` (1.0), `probabilistic` (0.5), `async-nondeterministic` (0.2).
- **side_effect_score**: `none` (1.0), `explicit` (0.6), `high` (0.2).
- **state_score**: `stateless` (1.0), `explicit-state` (0.8), `event-driven` (0.6), `async-io` (0.4), `external-boundary` (0.2).

---

### 2.3 Token Efficiency — T

Measures the retrieval density of the module.

```
T = 1.0 - (header_tokens / total_tokens)
```

Higher metadata density (ratio of header to implementation) is rewarded up to a point where implementation length doesn't bloat the retrieval process.

---

### 2.4 Performance — P

Measures algorithmic efficiency.

```
P = comp_map[@complexity]
```

| @complexity | P |
|---|---|
| O(1) | 1.0 |
| O(log n) | 0.9 |
| O(n) | 0.8 |
| O(n log n) | 0.6 |
| O(n²) | 0.4 |
| O(2ⁿ) | 0.1 |
| O(n!) | 0.0 |

---

### 2.5 Structural Quality — Q

Measures completeness of the metadata contract.

```
Q = min(1.0, (present_fields / 25) + bonus)
```

Includes bonus for optional fields: `@summary`, `@module`, `@usage-metrics`.

---

### 2.6 Isolation & Usage — I

Measures self-containment and ecosystem impact.

```
I = min(1.0, (depth_score + framework_score) / 2 + usage_bonus)
```

- **depth_score**: `1 / (1 + @dependency-depth)`.
- **framework_score**: 1.0 if `@framework-agnostic: true`.
- **usage_bonus**: `min(0.2, dependents * 0.01)`.

---

## 3. Grade Bands

| C | grade | meaning |
|---|---|---|
| 0.90 – 1.00 | A | Marketplace-ready. |
| 0.75 – 0.89 | B | Minor gaps. |
| 0.60 – 0.74 | C | Review required. |
| 0.40 – 0.59 | D | Refactor. |
| 0.00 – 0.39 | F | Invalid. |
