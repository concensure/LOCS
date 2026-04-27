# LOCS Module Registry (v1.2)

<!-- LOCS-REGISTRY-SCHEMA
format: markdown-table
parser: locs-registry-v1.2
automated-writes: true
write-trigger: module-created | module-modified | module-deprecated
collision-check: module-id must be unique
delete-policy: never-delete — set stability=deprecated instead
row-format: | module-id | module-name | domain | primary-capability | signature | version | stability | file-path |
placeholder-row: | _no entries yet_ | | | | | | | |
dependency-row-format: | module-id | depends-on | type |
changelog-row-format: | YYYY-MM-DD | module-id | change |
-->

---

## Registry Rules

- One row per module.
- `@module-id` must be unique — automated collision check runs before every write.
- `stability` must match current `@stability` in module metadata.
- Never delete rows — set `stability` to `deprecated` instead.
- All writes are performed automatically by the LOCS pipeline — no manual edits required.

---

## Registry Table

| module-id | module-name | domain | primary-capability | signature | version | stability | file-path |
|---|---|---|---|---|---|---|---|
| _no entries yet_ | | | | | | | |

---

## Stability Reference

| value | meaning |
|---|---|
| `stable` | Production-ready. Breaking changes require version bump. |
| `experimental` | API may change. Do not use in production flows. |
| `deprecated` | Superseded. Do not use in new modules. Keep row for audit. |

---

## Dependency Map

| module-id | depends-on | type |
|---|---|---|
| _no entries yet_ | | |

---

## Changelog

| date | module-id | change |
|---|---|---|
| _no entries yet_ | | |
