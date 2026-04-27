# LOCS Module Validator (v1.2)

Use this checklist to validate a module before registration. 

---

## 1. Metadata Header (Section 1)

- [ ] `@locs-version: 1.1` is present.
- [ ] `@key: value` syntax is strictly followed (colon required).
- [ ] `@module-id` follows `<domain>.<verb-noun>` format.
- [ ] `@primary-capability` and `@sub-capabilities` are defined and atomic.
- [ ] `@token-metrics` block is present and populated (header, implementation, ratio).
- [ ] `@capability-score` is present and grade is ≥ C.
- [ ] No prose paragraphs in metadata.
- [ ] Header is ≤ 50 lines.

---

## 2. Public Interface (Section 2)

- [ ] All exported functions and types are declared.
- [ ] Section contains **ZERO** implementation logic.
- [ ] Section follows immediately after metadata.

---

## 3. Behaviour Contract (Section 3)

- [ ] Clear declarative guarantees (Pure, Side Effects, Mutation).
- [ ] Determinism class matches `@determinism` field.

---

## 4. Implementation Consistency (Section 4)

- [ ] **Static Consistency**: All `@inputs` names are physically present in implementation signatures.
- [ ] **Static Consistency**: All `@outputs` types are present in the file.
- [ ] File is ≤ 400 LOC.
- [ ] Individual functions are ≤ 50 LOC.
- [ ] No undeclared internal dependencies.

---

## 5. Registry Integrity (Section 5)

- [ ] All internal dependencies listed in `@dependencies` exist in `LOCS_REGISTRY.md`.
- [ ] Module ID does not collide with existing registry entries.

---

## Validation Command

```bash
locs validate <file_path>
```
The CLI tool automates 90% of this checklist. Manual review is only required for semantic capability boundaries.
