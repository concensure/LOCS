# LOCS Session Init (v1.4)

Paste this file into an LLM session to activate LOCS governance.

---

## Workflow

```bash
# 1. Scaffold
python locs.py new <id>

# 2. Implement

# 3. Score and validate
python locs.py score <file> --write
python locs.py validate <file>

# 4. Register locally by default
python locs.py register <file>

# 5. Optional shared publication
python locs.py register <file> --scope shared

# 6. Bootstrap compact context
python locs.py bootstrap --category <slug> --limit 5
```

---

## Session Rules

- Prefer local registry routing first.
- Use shared registry only when cross-project reuse matters.
- Treat token metrics as backend-specific.
- Trust AST-backed validation over regex fallback when available.
- Load implementations only after registry and metadata routing.
