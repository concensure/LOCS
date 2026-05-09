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

## 3. Implementation Phases
Refer to `tasks.md` for the detailed implementation roadmap.
