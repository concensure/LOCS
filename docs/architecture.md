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
