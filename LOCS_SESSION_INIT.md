# LOCS Session Init (v2.0)

Paste this file into an LLM session to activate LOCS governance.

---

## Workflow

```bash
# 1. Scaffold
locs new <id>

# 2. Implement

# 3. Score and validate
locs score <file> --write
locs validate <file>
# validate prints AST backend and token backend so you know validation confidence
# Fix ALL failures before proceeding — do not git add a file that fails validation

# 4. Stage only after validation passes
git add <file>

# 5. Register locally by default
locs register <file>
# registration updates .locs.index.json for fast bootstrap

# 6. Optional shared publication
locs register <file> --scope shared

# 7. Bootstrap compact context (uses index when available)
locs bootstrap --category <slug> --limit 5

# 8. Rebuild index if needed
locs index rebuild

# Discover ungoverned capability files (run in CI or on demand — never blocks commits)
locs audit
locs audit --format json --exit-nonzero   # CI mode: exits 1 if any found
```

---

## Session Rules

- Prefer local registry routing first.
- Use shared registry only when cross-project reuse matters.
- Treat token metrics as backend-specific — only compare counts from the same backend family.
- Trust AST-backed validation (exact) over regex fallback; check the confidence line in validate output.
- Load implementations only after registry and metadata routing.
- Stability follows the v2 lifecycle: draft → active → stabilising → stable → protected → frozen.
- `locs validate` is a generation-time gate, not a commit-time gate. Run it before `git add`.
- `locs audit` is informational only — it never blocks a commit. Use it in CI or on demand.
- Never install the pre-commit hook globally. Use `locs hook install` to scope it to the current repo.
