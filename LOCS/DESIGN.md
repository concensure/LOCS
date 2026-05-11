# LOCS v2 Design Philosophy

## 1. Vision
LOCS (Library of Capabilities and Signatures) provides a formal, machine-readable contract between source code and autonomous agents. v2 focuses on **deterministic governance**, ensuring that LLM agents operate within safe, verifiable boundaries.

## 2. Core Principles
- **Machine-Readable Intent**: Metadata should be dense, deterministic, and easily extracted by both simple regex and robust AST parsers.
- **Truth in Advertising**: Implementation must match the declared capabilities. Governance tools (like GuardPatch) verify this consistency.
- **Stability-Aware**: Code and documentation have a lifecycle. Governance rigor increases as a component matures.
- **Language Agnostic**: The contract format (LOCS metadata) should be consistent across Python, Rust, TypeScript, and Go.

## 4. Section-Based Addressing and Governance
While AST parsing provides fine-grained control over code, documentation and logical code regions require a more flexible, stable addressing mechanism.

- **Stable Anchors**: Section IDs provide refactoring-resilient targets for governance and retrieval.
- **Sparse Metadata**: Governance should be "sparse" — applied only at significant boundaries to minimize noise and token bloat.
- **Unified Schema**: Whether in Markdown or Code, the metadata schema remains consistent (`id`, `edit`, `role`).
- **Low-Friction Inference**: "Ghost Inference" uses project conventions to provide high-quality defaults without manual annotation.
- **Sidecar Governance**: Shadow policies allow governing legacy or third-party code without source modification.
