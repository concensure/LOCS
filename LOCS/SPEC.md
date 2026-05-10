# LOCS v2 Specification (Draft)

## 1. Metadata Format
LOCS metadata is embedded in source files using language-appropriate comment blocks or frontmatter.

### 1.1 Key-Value Syntax
Use a flat `@key: value` pattern for maximum token efficiency and ease of parsing.

```yaml
LOCS:
  capability: <short-capability-name>
  stability: draft|active|stabilising|stable|protected|frozen
  owner: <identity>
  kind: cli|parser|policy|verifier|report|test|design|spec
guard:
  mode: editable|proposal_only|review_required|protected|frozen|append_only|generated|human_only|deprecated
  lock_signature: boolean
  lock_body: boolean
  require_tests: boolean
  unlock_requires: string
```

## 2. Stability Levels
- `draft`: Experimental, high-velocity, low governance.
- `active`: Maintained, standard governance.
- `stabilising`: Feature complete, evidence collection in progress.
- `stable`: Breaking changes require high ceremony.
- `protected`: No edits allowed without explicit unlock.
- `frozen`: Immutable for the current version/cycle.

## 3. Guard Modes
- `editable`: Standard AI-assisted editing allowed.
- `proposal_only`: Edits are staged for human review, never applied directly.
- `review_required`: Changes trigger a mandatory human approval step.
- `protected`: Rejected by default; requires an `unlock` command.
- `human_only`: Only human actors (non-LLM) can modify.
- `generated`: Managed by a tool; manual edits discouraged/blocked.
