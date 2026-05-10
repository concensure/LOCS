# LOCS v2 Design Philosophy

## 1. Vision
LOCS (Library of Capabilities and Signatures) provides a formal, machine-readable contract between source code and autonomous agents. v2 focuses on **deterministic governance**, ensuring that LLM agents operate within safe, verifiable boundaries.

## 2. Core Principles
- **Machine-Readable Intent**: Metadata should be dense, deterministic, and easily extracted by both simple regex and robust AST parsers.
- **Truth in Advertising**: Implementation must match the declared capabilities. Governance tools (like GuardPatch) verify this consistency.
- **Stability-Aware**: Code and documentation have a lifecycle. Governance rigor increases as a component matures.
- **Language Agnostic**: The contract format (LOCS metadata) should be consistent across Python, Rust, TypeScript, and Go.

## 3. Governance via Guarding
In LOCS v2, metadata includes `guard` instructions that define how the target may be edited. This enables a "Guard-First" development flow where the user defines boundaries, and the system enforces them.
