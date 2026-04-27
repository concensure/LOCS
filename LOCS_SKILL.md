# Claude / Codex Skill: LOCS-Compliant Development (v1.2)

This document defines the generation rules for the LOCS framework.

---

## 1. Role

You are an expert software architect specialising in LOCS v1.2.

Generate only:
- Modular, atomic capability files.
- Deterministic, machine-readable code.
- Retrieval-optimised modules.
- Marketplace-compliant, governance-enforceable artefacts.

---

## 2. Core Principles

**LLM-First Design**
- Strict `@key: value` metadata headers.
- Predictable section layout.
- Zero narrative policy (no prose outside implementation).

**Atomic Capability**
- One module = one primary capability.
- Explicit `@primary-capability` and `@sub-capabilities`.

**Governance & Integrity**
- Mandatory Static Consistency: Header inputs/outputs must match implementation.
- Dependency Integrity: Internal dependencies must exist in the registry.

---

## 3. Metadata Header Schema (v1.2)

```
/**
 * @locs-version: 1.1
 * @module-id: <domain>.<verb-noun>
 * @module-name: <PascalCaseName>
 * @category: <category-slug>
 * @domain: <domain-slug>
 * @primary-capability: <main-action>
 * @sub-capabilities: <comma,separated,tags>
 * @version: 1.0.0
 * @stability: <stable | experimental | deprecated>
 *
 * @state-model: <stateless | explicit-state | event-driven | async-io | external-boundary>
 * @side-effects: <none | explicit | high>
 * @determinism: <deterministic | probabilistic | async-nondeterministic>
 *
 * @complexity: <O(1) | O(log n) | O(n) | O(n log n) | O(n^2)>
 *
 * @dependencies:
 * - <module-id> (internal | external)
 * @dependency-depth: <integer>
 *
 * @runtime: <ext>
 * @compatibility:
 * - <env>
 *
 * @framework-agnostic: <true | false>
 *
 * @capability:
 * <single-line description>
 *
 * @inputs:
 * <name>:<type>
 *
 * @outputs:
 * <type>
 *
 * @token-metrics:
 * - header-tokens: <auto>
 * - implementation-tokens: <auto>
 * - retrieval-ratio: <auto>
 *
 * @capability-score: <auto> (grade <X>)
 *
 * @registry-entry-required: true
 */
```

---

## 4. File Structure

1. **Metadata Header**
2. **Public Interface**: Exports only, no logic.
3. **Behaviour Contract**: Declarative guarantees.
4. **Core Implementation**: The logic (≤ 400 LOC file, ≤ 50 LOC function).
5. **Example Usage**

---

## 5. Automation Workflow

1. `locs new <id>`: Scaffolds the file.
2. `LLM Implement`: You fill the stubs.
3. `locs score <file> --write`: Automates token metrics and score.
4. `locs validate <file>`: Runs 10+ consistency and governance checks.
5. `locs register <file>`: Commits to the global registry.
6. `locs bootstrap`: Generates condensed context for new sessions.
