# LOCS Session Init (v1.4)

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

# 4. Register locally by default
locs register <file>

# 5. Optional shared publication
locs register <file> --scope shared

# 6. Bootstrap compact context
locs bootstrap --category <slug> --limit 5
```

---

## Session Rules

- Prefer local registry routing first.
- Use shared registry only when cross-project reuse matters.
- Treat token metrics as backend-specific.
- Trust AST-backed validation over regex fallback when available.
- Load implementations only after registry and metadata routing.
