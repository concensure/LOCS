# LOCS Grand Module Registry (v1.4)

<!-- LOCS-REGISTRY-SCHEMA
format: markdown-table
parser: locs-registry-v1.4
scope: shared
automated-writes: true
write-trigger: module-created | module-modified | module-deprecated
collision-check: module-id must be unique
delete-policy: never-delete - set stability=deprecated instead
row-format: | module-id | module-name | category | domain | primary-capability | signature | version | stability | file-path | capability-score |
placeholder-row: | _no entries yet_ | | | | | | | | | |
dependency-row-format: | module-id | depends-on | type |
changelog-row-format: | YYYY-MM-DD | module-id | change |
-->

---

## Grand Registry Scope

This registry is optional. Use it only when capability units need to be shared across projects or when you want a workspace-level marketplace.

For normal per-project development, prefer `LOCS_REGISTRY.md`.

---

## Registry Rules

- Shared scope is opt-in.
- `@module-id` must be unique inside the shared registry.
- Shared entries use absolute file paths so any project can retrieve them.
- One row per module, written by `locs register --scope shared`.

---

## Registry Table

| module-id | module-name | category | domain | primary-capability | signature | version | stability | file-path | capability-score |
|---|---|---|---|---|---|---|---|---|---|
| _no entries yet_ | | | | | | | | | |

---

## Dependency Map (Cross-Project)

| module-id | depends-on | type |
|---|---|---|
| _no entries yet_ | | |

---

## Global Changelog

| date | module-id | change |
|---|---|---|
| 2026-04-28 | registry.init | Transitioned to optional shared registry model. |
