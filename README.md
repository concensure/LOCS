# LOCS — LLM-Optimised Capability Specification (v1.2)

**LOCS** is a portable framework for writing code that LLMs can reliably read, retrieve, compose, and govern. It transforms a standard codebase into a machine-readable "Capability Marketplace."

---

## The Problem: Context Exhaustion & Retrieval Noise

Modern LLMs have large context windows, but they are still limited by **Effective Attention** and **Token Cost**. When an LLM interacts with a traditional codebase, it faces three critical failures:

1.  **The "Blind Load" Problem:** To find a single function, an LLM often has to load entire files. This fills the context window with implementation noise before the model even begins reasoning.
2.  **Semantic Ambiguity:** Files named `utils.ts` or `helpers.py` provide zero signal to the model's retrieval engine, leading to "wrong-file" calls and hallucinated dependencies.
3.  **High Retrieval Latency:** Without a machine-readable index, a model must "guess" where a capability lives, leading to repeated turns and wasted tokens.

**In short: Codebases are currently written for human eyes, not LLM context windows.**

---

## The Solution: Selective Retrieval Architecture

LOCS solves these problems by enforcing a **"Declare-before-implement"** architecture. It treats every code module as a **Capability Unit**—a self-describing, atomic artefact.

### 1. O(1) Routing via Registry
The `LOCS_REGISTRY.md` provides a dense "Semantic Map" of the repo. By using the `locs bootstrap` command, you can inject the entire repo's capabilities into an LLM session in ~200 tokens. The model can then select the exact module it needs without opening a single file.

### 2. High-Density Metadata
The `@key: value` metadata header allows the LLM to evaluate the utility, safety, and complexity of a module within the first 50 lines. It can decide to "skip" or "use" a module with 90% less token consumption than a full file load.

### 3. Governance as a Moat
LOCS doesn't just suggest a style; it enforces it. Through the `locs validate` command, the framework ensures that what is promised in the metadata is actually delivered in the code (Static Consistency), making the codebase a "High-Trust" environment for autonomous agents.

---

## How it Works

```mermaid
graph TD
    A[Developer / LLM] -->|locs new| B(Scaffolded Module)
    B -->|LLM Implementation| C(Implemented Module)
    C -->|locs score --write| D(Metric-Rich Module)
    D -->|locs validate| E{Valid?}
    E -- No -->|Fix| C
    E -- Yes -->|locs register| F[(LOCS_REGISTRY.md)]
    F -->|locs bootstrap| G[LLM Context/Session]
    G -->|Architecture Awareness| A
```

---

## Installation

**One-line install (from project root):**
```bash
pip install -e .
```
This installs the `locs` CLI tool globally (or in your active venv) and allows you to use the framework across any directory in your project.

---

## Usage Workflow

```bash
# 1. Scaffold a new module (multi-language support)
locs new domain.verb-noun --ext .py

# 2. Score and automate token metrics
locs score src/module.py --write

# 3. Validate governance rules
locs validate src/module.py

# 4. Register in global index
locs register src/module.py

# 5. Bootstrap a new LLM session
locs bootstrap --category graph-routing
```

---

## File Reference

- `locs.py`: CLI engine (v1.2).
- `LOCS_REGISTRY.md`: Global capability index.
- `LOCS_SKILL.md`: Generation rules & schema.
- `LOCS_SESSION_INIT.md`: LLM bootstrap file.
- `LOCS_CAPABILITY_SCORING.md`: Six-dimension scoring algorithm.
- `docs/architecture.md`: Detailed framework design.

---

## Token Efficiency

LOCS is designed to save tokens over the long term by avoiding "wrong-file" loads and providing dense semantic signal at the registry level.

| Operation | v1.1 Cost | v1.2 Cost | Improvement |
|---|---|---|---|
| Registry Routing | Medium | Low | Condensed signatures via bootstrap |
| Header Parsing | High | Medium | Strict @key: value syntax |
| Implementation Load | Variable | Lower | Complexity & token-ratio signal |
