# Proposed: Practical LOCS Enforcement for AI-Assisted Coding

## Assessment of the Existing Strategy

The [existing strategy](automatic_enforcement_strategy.md) is well-intentioned but designed with human-paced coding in mind. Several suggestions become friction multipliers or token sinks when the primary developer is an AI coding assistant (Claude Code, Codex, Cursor). This document calls out the specific problems and proposes a leaner alternative.

---

## Problems with the Existing Approach

### 1. The "Smell Test" fires constantly under AI-generated code

The proposed triggers — 300 LOC, 3 exported classes, 5 exported functions — are calibrated for human-written files. An AI assistant routinely generates fully-formed, well-structured modules that exceed all three thresholds in a single turn. The result is that **every meaningful commit would be blocked**, forcing a manual resolution step after each generation cycle. That is the opposite of productivity.

### 2. `locs decompose` puts LLM calls in the commit path

Sending signatures to an LLM at commit time introduces 5–30 seconds of latency per commit and burns tokens every time a sufficiently complex file is touched. Commit hooks must be fast and local. An API call in a pre-commit hook is a hard blocker for offline work, rate-limited environments, and CI runners. The "low-token prompt" framing doesn't change the fundamental problem: any network call in a commit hook is a liability.

### 3. Global git template installation is too invasive

A user-level git template applies to **every new repository the developer creates**, not just LOCS projects. The existing strategy acknowledges the "security breach" concern but then proposes exactly that. A repo-scoped hook installed by `locs init` is the correct scope. Global templates are appropriate for universal tooling (commit linting, secret scanning), not framework-specific validators.

### 4. Score-on-Change requires persistent state outside git

Tracking whether a file's complexity increased by 20% across commits requires either a sidecar database or a git-tracked metadata file. A sidecar database is non-portable. A git-tracked file creates merge conflicts whenever two branches touch the same module. Neither is worth the marginal governance gain.

### 5. GuardPatch auto-mutation risks conflict noise

Auto-updating `.guardpatch.yml` on `locs new` means the config file changes on every scaffold operation. In a multi-branch workflow this generates spurious conflicts and makes the config harder to reason about. Governance config should be explicit, not auto-generated.

---

## The Correct Mental Model for AI Workflows

The existing pre-commit hook (in `hooks/pre-commit`) is already correct in its design:

- It only validates files that already carry `@locs-version`.
- It uses a fast `grep` scan, not AST parsing.
- It blocks commits only for files that opted into the framework but failed validation.
- It exits cleanly when no LOCS files are staged.

**This is the right boundary.** The pre-commit hook should remain the last line of defense against *broken* LOCS headers — not the first line of defense against *missing* ones.

The missing coverage — files that should be LOCS modules but aren't annotated yet — belongs in a different layer entirely.

---

## Proposed Strategy

### Principle 1: Enforce at generation time, not commit time

The most effective enforcement for AI coding workflows is the **session context**. `LOCS_SKILL.md` already does this: when loaded by the AI, it defines the generation rules, and the AI follows them natively before writing a single line. Reinforcing this in `CLAUDE.md` with a `locs validate <file>` step before registering any module costs zero tokens at commit time and catches issues before they exist.

The pre-commit hook exists to catch mistakes the session context missed. It should stay cheap.

**Practical addition:** Ensure `CLAUDE.md` in every LOCS-governed repo includes:
```
- Run `locs validate <file>` before staging any new capability module.
- Do not stage a file with @locs-version unless validation passes.
```
No new tooling needed. The AI follows the rule.

### Principle 2: Keep the pre-commit hook grep-based and local

Do not replace the `grep -q "@locs-version"` scan with AST parsing. The current approach is correct:

- `grep` is O(n) on file size, completes in milliseconds, has no dependencies.
- AST parsing (Python `ast`, tree-sitter) is slower and requires runtime dependencies that may not be present on all machines or CI runners.
- The hook's job is to validate *declared* LOCS modules, not to discover undeclared ones.

If tree-sitter or `ast` is needed for `locs validate` (already used by the Python CLI), that cost is paid once during `locs validate`, not on every commit.

### Principle 3: Move capability discovery to CI, not pre-commit

The smell-test heuristics belong in a CI step, not a blocking pre-commit hook. A non-blocking CI check that runs `locs audit --unprotected` after a push:

- Does not interrupt the developer's local commit flow.
- Can post a PR comment listing files that look like capabilities but lack headers.
- Can be configured to block merges to `main` (not local commits) if ungoverned capability files accumulate past a threshold.
- Is trivially skippable for branches tagged `wip/` or `draft/`.

This separates the **fast, local, blocking** concern (broken headers) from the **async, non-blocking, advisory** concern (missing headers).

### Principle 4: Auto-install stays repo-local

`locs init` should install the hook only in the current repo's `.git/hooks/`. No global git templates. If the developer wants cross-repo enforcement, they run `locs init` in each repo. The hook is already 53 lines of shell — it's trivial to install manually.

The contextual prompt (`locs hook install`) from the existing proposal is a good UX pattern — preserve it, but scope it to the current repo only.

### Principle 5: `locs audit` is the governance surface, not the commit surface

`locs audit --unprotected` is already a proposed command. It is the right tool for discovering ungoverned capabilities. The right triggers for running it:

| Trigger | Mode | Blocks? |
|---|---|---|
| `git push` to `main` | CI step | Optional (configurable) |
| PR opened | CI step | Optional (configurable) |
| Manual: `locs audit` | Developer-initiated | Never |
| Pre-commit | Never | — |

This avoids the "pre-commit as governance" antipattern while still giving teams a hard enforcement surface at merge time.

### Principle 6: No LLM calls in any automated pipeline step

Any step that runs automatically (pre-commit, CI) must be 100% local. LLM calls belong only in developer-initiated commands (`locs decompose`, `locs score`, `locs new`). The reason is simple: automated steps need to be deterministic, fast, free, and offline-capable. LLM calls are none of those things by default.

`locs decompose` as an interactive, developer-invoked tool is still a useful idea — but it should never run automatically.

---

## Summary: What to Keep, What to Drop, What to Add

| Idea from existing strategy | Verdict | Reason |
|---|---|---|
| Intent-aware "Smell Test" in pre-commit | Drop | Fires constantly for AI-generated code; breaks local flow |
| Zero-token AST verification in pre-commit | Keep concept, current grep is sufficient | AST is already used by `locs validate`; no need to duplicate in hook |
| `locs decompose` as interactive command | Keep | Useful, but never run automatically |
| `locs decompose` at commit time | Drop | LLM call in commit path = latency + token burn |
| Global git template auto-install | Drop | Too invasive; affects all repos |
| `locs hook install` contextual prompt | Keep | Good UX, keep it repo-scoped |
| GuardPatch auto-update on `locs new` | Drop | Creates merge noise; keep config explicit |
| `locs audit --unprotected` | Keep | Move to CI, not pre-commit |
| Score-on-Change 20% trigger | Drop | Requires persistent state, creates conflict risk |
| Session-level enforcement via LOCS_SKILL.md | Strengthen | Primary enforcement layer for AI coding |

---

*Proposed as a counterpoint to `automatic_enforcement_strategy.md` (Gemini CLI draft).*
