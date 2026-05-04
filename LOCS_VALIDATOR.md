# LOCS Module Validator (v1.4)

Use this checklist before registration.

---

## 1. Metadata Header

- [ ] `@locs-version: 1.2` is present.
- [ ] Strict `@key: value` syntax is used.
- [ ] `@module-id` follows `<domain>.<verb-noun>`.
- [ ] `@primary-capability` and `@sub-capabilities` are atomic.
- [ ] `@token-metrics` is populated, including tokenizer backend.
- [ ] `@capability-score` is populated and grade is `A`, `B`, or `C`.
- [ ] Header is 80 non-empty lines or less.

## 2. Token Backend

- [ ] OpenAI counts use `tiktoken` when exact counts are desired.
- [ ] Hugging Face counts use `transformers` tokenizers when exact counts are desired.
- [ ] SentencePiece counts use a real model file when exact counts are desired.
- [ ] Heuristic fallback is acceptable only when no exact backend is available.

## 3. Implementation Consistency

- [ ] Python modules pass built-in AST checks.
- [ ] JavaScript/TypeScript modules use Tree-sitter checks when installed.
- [ ] Declared `@inputs` appear in parsed signatures.
- [ ] Declared `@outputs` are represented in code.
- [ ] `@side-effects` matches actual behavior.

## 4. Dependency Integrity

- [ ] Internal dependencies exist in the selected registry.
- [ ] No circular dependency is introduced.
- [ ] `@dependency-depth` matches computed depth.
- [ ] Dependency depth stays within the cap.

## 5. Registry Scope

- [ ] Use `LOCS_REGISTRY.md` for normal project routing.
- [ ] Use `LOCS_GRAND_REGISTRY.md` only for intentional cross-project sharing.

---

## Commands

```bash
locs score <file> --write
locs validate <file>
locs register <file>

# Optional shared registry
locs register <file> --scope shared
```
