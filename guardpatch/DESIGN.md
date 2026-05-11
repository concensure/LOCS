# GuardPatch Design

## 1. Architecture
GuardPatch follows a modular architecture:
- **CLI**: Entry point.
- **Policy**: Resolves what is allowed.
- **Parse**: Understands file content and structure.
- **Patch**: Normalizes diffs into internal operations.
- **Core**: The decision engine.
- **Audit**: Logging and reporting.

## 2. Verification Model
Verification is stateless and deterministic. It compares a "Candidate State" (in-memory) against the "Policy State".

## 4. Governance Boundaries
GuardPatch enforces boundaries at multiple levels:
- **Global/Path**: Defined in `.guardpatch.yml`.
- **File-Level**: Defined in LOCS frontmatter/metadata.
- **Section-Level**: Defined via LOCS Section Addressing:
  - **Markdown**: `## Title <!-- locs:id=... locs:edit=... -->`
  - **Code**: `/* locs:section id=... edit=... */`
- **AST-Level**: Automatic detection of symbols (functions, classes) for supported languages.
