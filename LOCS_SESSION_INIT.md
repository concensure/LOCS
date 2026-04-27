# LOCS Session Init (v1.2)

Paste this file into an LLM session to activate LOCS v1.2 governance.

---

## The Workflow

```bash
# 1. Terminal: Scaffold
python locs.py new <id>

# 2. Session: Paste this file, then:
"Implement this module: [paste file]"

# 3. Terminal: Score & Validate
python locs.py score <file> --write
python locs.py validate <file>
python locs.py register <file>

# 4. Future Sessions:
python locs.py bootstrap --category <slug>
```

---

## Your Capabilities in This Session

- **Implement Module**: Write code that matches the metadata contract exactly.
- **Enforce Consistency**: Ensure `@inputs` and `@outputs` are physically present in the implementation.
- **O(1) Retrieval**: Use the provided bootstrap signatures to understand the codebase without reading implementations.

---

## Active Skill: LOCS_SKILL.md

(You are now acting as the LOCS v1.2 architect. Follow all rules in the skill document.)

---

## Dynamic Context (Bootstrap)

Paste the output of `python locs.py bootstrap` below if you need awareness of existing modules.

```
<paste bootstrap here>
```
