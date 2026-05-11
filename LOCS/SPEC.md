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
## 4. Section Addressing
Stable, deterministic governance boundaries for documents and source code.

### 4.1 Markdown Anchors
Use heading-attached HTML comments for stable identification and inline edit policy.
Syntax: `## Title <!-- locs:id=stable-id locs:edit=mode locs:role=role -->`

- `locs:id`: Stable identifier (recommended for all significant headings).
- `locs:edit`: `locked` | `editable` | `approval`
- `locs:role`: `metadata` | `contract` | `implementation` | `example` | `notes`

### 4.2 Code Section Markers
Use block or line comments to define logical regions where AST alone is insufficient.
Syntax (TypeScript/JS/Rust/Go):
```typescript
/* locs:section id=impl edit=editable role=implementation */
// ... content ...
/* locs:end */
```
Syntax (Python/Shell):
```python
# locs:section id=impl edit=editable role=implementation
# ... content ...
## 5. Governance Enhancements

### 5.1 Ghost Inference
Governance roles can be inferred from file paths if not explicitly declared in the file.
Config example:
```yaml
role_inference:
  - pattern: "tests/**"
    role: example
```

### 5.2 Evidence Mapping
Roles are linked to mandatory evidence commands.
```yaml
evidence_map:
  - role: contract
    commands: ["cargo test", "cargo clippy"]
```

### 5.3 Shadow Policy (Sidecars)
External governance policies can be applied via `.guardpatch.sidecar.yml` for read-only or legacy files.

### 5.4 Self-Stabilizing Headers (Seal/Unseal)
CLI commands to automate the generation and removal of stable section anchors.
- `guardpatch seal --path <f>`: Appends deterministic IDs to all headings.
- `guardpatch unseal --path <f>`: Removes all LOCS anchors.
