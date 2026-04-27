#!/usr/bin/env python3
"""
LOCS CLI (v1.1)
Commands: new | score | validate | register | status
stdlib only — no dependencies
"""

import argparse
import json
import math
import os
import re
import sys
from datetime import date
from pathlib import Path

# ─── CONSTANTS ────────────────────────────────────────────────────────────────

REGISTRY_FILE = "LOCS_REGISTRY.md"

REQUIRED_FIELDS = [
    "locs-version", "module-id", "module-name", "category", "domain",
    "primary-capability", "sub-capabilities", "version", "stability",
    "state-model", "side-effects", "determinism", "complexity",
    "dependency-depth", "runtime", "framework-agnostic", "capability",
    "inputs", "outputs", "preconditions", "postconditions",
    "use-when", "avoid-when", "token-metrics", "registry-entry-required",
]

# optional bonus fields — lift Q when present, never penalise absence
OPTIONAL_FIELDS = ["summary", "module", "usage-metrics"]

VALID_STABILITY    = {"stable", "experimental", "deprecated"}
VALID_STATE_MODEL  = {"stateless", "explicit-state", "event-driven", "async-io", "external-boundary"}
VALID_SIDE_EFFECTS = {"none", "explicit", "high"}
VALID_DETERMINISM  = {"deterministic", "probabilistic", "async-nondeterministic"}

# Multi-language comment styles
COMMENT_STYLES = {
    ".ts": ("/**", " *", " */"),
    ".js": ("/**", " *", " */"),
    ".py": ("\"\"\"", "", "\"\"\""),
    ".go": ("/*", " *", " */"),
    ".rs": ("/*", " *", " */"),
    ".java": ("/**", " *", " */"),
    ".c": ("/*", " *", " */"),
    ".cpp": ("/*", " *", " */"),
}

DEFAULT_HEADER_TEMPLATE = """\
{comment_start}
{prefix} @locs-version: 1.1
{prefix} @module-id: {module_id}
{prefix} @module-name: {module_name}
{prefix} @category: {category}
{prefix} @domain: {domain}
{prefix} @primary-capability: {primary_capability}
{prefix} @sub-capabilities: {sub_capabilities}
{prefix} @version: 1.0.0
{prefix} @stability: stable
{prefix}
{prefix} @state-model: stateless
{prefix} @side-effects: none
{prefix} @determinism: deterministic
{prefix}
{prefix} @complexity: O(n)
{prefix}
{prefix} @dependencies:
{prefix} - none
{prefix} @dependency-depth: 0
{prefix}
{prefix} @runtime: {runtime}
{prefix} @compatibility:
{prefix} - node>=18
{prefix} - browser: yes
{prefix} - esm: true
{prefix}
{prefix} @framework-agnostic: true
{prefix}
{prefix} @summary:
{prefix} <1-2 sentence plain-English description of the problem this solves>
{prefix}
{prefix} @module: <parent-module> (omit if top-level)
{prefix}
{prefix} @capability:
{prefix} <single-line capability description>
{prefix}
{prefix} @inputs:
{prefix} <name>:<type>
{prefix}
{prefix} @outputs:
{prefix} <type>
{prefix}
{prefix} @preconditions:
{prefix} - <condition>
{prefix}
{prefix} @postconditions:
{prefix} - <guarantee>
{prefix}
{prefix} @use-when:
{prefix} - <scenario>
{prefix}
{prefix} @avoid-when:
{prefix} - <scenario>
{prefix}
{prefix} @token-metrics:
{prefix} - header-tokens: 0
{prefix} - implementation-tokens: 0
{prefix} - retrieval-ratio: 0.0
{prefix}
{prefix} @usage-metrics:
{prefix} - dependents: 0
{prefix} - usage-tier: niche
{prefix}
{prefix} @capability-score: auto
{prefix} @benchmark-ref: optional
{prefix}
{prefix} @registry-entry-required: true
{comment_end}

{interface_stub}

{contract_stub}

{implementation_stub}

{example_stub}
"""

# ─── METADATA PARSER ─────────────────────────────────────────────────────────

def estimate_tokens(text: str) -> int:
    """Heuristic token counter (words * 1.3 + character density)."""
    words = text.split()
    word_count = len(words)
    char_count = len(text)
    # Average word length in English is ~5 characters.
    # 1.3 tokens per word is a common heuristic for LLMs.
    return int(max(word_count * 1.3, char_count / 4))

def parse_metadata(path: Path) -> dict:
    """Extract @field values from the LOCS header block."""
    text = path.read_text(encoding="utf-8")
    meta = {}

    # Detect comment style
    ext = path.suffix
    styles = COMMENT_STYLES.get(ext, ("/*", " *", " */"))
    start, prefix, end = styles

    # pull the comment block - use re.escape for the start/end tokens
    pattern = rf"{re.escape(start)}(.*?){re.escape(end)}"
    block_match = re.search(pattern, text, re.DOTALL)
    if not block_match:
        return meta

    block = block_match.group(1)

    # single-line fields: @key: value or @key value
    for line in block.splitlines():
        line = line.strip()
        if prefix and line.startswith(prefix.strip()):
            line = line[len(prefix.strip()):].strip()
        
        m = re.match(r"@([\w-]+)[:\s]+(.*)", line)
        if m:
            key, val = m.group(1), m.group(2).strip()
            if key not in meta:
                meta[key] = val

    # multi-line fields
    fields = [
        "capability", "inputs", "outputs", "preconditions",
        "postconditions", "use-when", "avoid-when", "token-metrics",
        "usage-metrics", "summary", "dependencies"
    ]
    for field in fields:
        # Match @field: followed by content until next @ or end of block
        pattern = rf"@{field}[:\s]*\n(.*?)(?=\n\s*{re.escape(prefix.strip())}\s*@|\*/|\"\"\")"
        m = re.search(pattern, block, re.DOTALL)
        if m:
            content = m.group(1)
            # Clean up the prefix from each line
            lines = []
            for l in content.splitlines():
                l = l.strip()
                if prefix and l.startswith(prefix.strip()):
                    l = l[len(prefix.strip()):].strip()
                lines.append(l)
            meta[field] = "\n".join(lines).strip()

    return meta

# ─── SCORING ─────────────────────────────────────────────────────────────────

def _pascal_word_count(name: str) -> int:
    return len(re.findall(r"[A-Z][a-z0-9]*", name))

def _capability_word_count(cap: str) -> int:
    return len(cap.split())

def compute_score(meta: dict, full_text: str = "") -> tuple[float, str]:
    """Returns (score, grade). Score in [0,1]."""

    # R — Retrieval Clarity (Weighted 0.25)
    subs = [s.strip() for s in meta.get("sub-capabilities", "").split(",") if s.strip()]
    sub_score = min(len(subs), 5) / 5
    
    primary = meta.get("primary-capability", "").strip()
    primary_score = 1.0 if primary else 0.0

    cap_text = meta.get("capability", "")
    L = _capability_word_count(cap_text)
    if 5 <= L <= 12:
        cap_score = 1.0
    elif L < 5:
        cap_score = L / 5
    else:
        cap_score = 12 / L

    name = meta.get("module-name", "")
    W = _pascal_word_count(name)
    if 2 <= W <= 4:
        name_score = 1.0
    elif W < 2:
        name_score = W / 2 if W > 0 else 0.0
    else:
        name_score = 4 / W

    R = (sub_score + primary_score + cap_score + name_score) / 4

    # D — Determinism & Safety (Weighted 0.20)
    det_map   = {"deterministic": 1.0, "probabilistic": 0.5, "async-nondeterministic": 0.2}
    side_map  = {"none": 1.0, "explicit": 0.6, "high": 0.2}
    state_map = {"stateless": 1.0, "explicit-state": 0.8, "event-driven": 0.6,
                 "async-io": 0.4, "external-boundary": 0.2}

    det_score   = det_map.get(meta.get("determinism", ""), 0.0)
    side_score  = side_map.get(meta.get("side-effects", ""), 0.0)
    state_score = state_map.get(meta.get("state-model", ""), 0.0)
    D = (det_score + side_score + state_score) / 3

    # T — Token Efficiency (Weighted 0.20)
    tm_block = meta.get("token-metrics", "")
    ratio = 0.1
    for line in tm_block.splitlines():
        if "retrieval-ratio" in line:
            parts = line.split(":")
            if len(parts) > 1:
                try: ratio = float(parts[1].strip())
                except: pass
    
    # Retrieval ratio: header tokens / total tokens. Lower is better for retrieval density.
    T = min(1.0, 1.0 - ratio)

    # P — Performance (Weighted 0.15)
    complexity = meta.get("complexity", "O(n)").upper()
    comp_map = {
        "O(1)": 1.0, "O(LOG N)": 0.9, "O(N)": 0.8, 
        "O(N LOG N)": 0.6, "O(N^2)": 0.4, "O(2^N)": 0.1, "O(N!)": 0.0
    }
    P = comp_map.get(complexity, 0.5)

    # Q — Structural Quality (Weighted 0.15)
    present = sum(1 for f in REQUIRED_FIELDS if meta.get(f, "").strip())
    base_Q = present / len(REQUIRED_FIELDS)
    bonus = sum(0.5 / len(REQUIRED_FIELDS) for f in OPTIONAL_FIELDS if meta.get(f, "").strip())
    Q = min(1.0, base_Q + bonus)

    # I — Isolation & Usage (Weighted 0.05)
    try: depth = int(meta.get("dependency-depth", "0"))
    except: depth = 0
    depth_score = 1 / (1 + depth)
    
    usage_block = meta.get("usage-metrics", "")
    dependents = 0
    for line in usage_block.splitlines():
        if "dependents" in line:
            m = re.search(r"(\d+)", line)
            if m: dependents = int(m.group(1))
    
    usage_bonus = min(0.2, dependents * 0.01) # Max 0.2 bonus
    I = min(1.0, (depth_score + (1.0 if meta.get("framework-agnostic") == "true" else 0.0)) / 2 + usage_bonus)

    C = 0.25 * R + 0.20 * D + 0.20 * T + 0.15 * P + 0.15 * Q + 0.05 * I
    C = round(C, 3)

    if C >= 0.90:   grade = "A"
    elif C >= 0.75: grade = "B"
    elif C >= 0.60: grade = "C"
    elif C >= 0.40: grade = "D"
    else:           grade = "F"

    return C, grade

# ─── VALIDATION ──────────────────────────────────────────────────────────────

def validate_module(path: Path, meta: dict) -> list[str]:
    """Returns list of failure messages. Empty = pass."""
    failures = []
    text = path.read_text(encoding="utf-8")

    # 1. Metadata completeness
    for f in REQUIRED_FIELDS:
        if not meta.get(f, "").strip():
            failures.append(f"[metadata] missing or empty: @{f}")

    # 2. Metadata line count
    ext = path.suffix
    styles = COMMENT_STYLES.get(ext, ("/*", " *", " */"))
    start, prefix, end = styles
    pattern = rf"{re.escape(start)}(.*?){re.escape(end)}"
    block_match = re.search(pattern, text, re.DOTALL)
    if block_match:
        lines = block_match.group(1).splitlines()
        if len(lines) > 50: # Increased from 40 to accommodate new fields
            failures.append(f"[metadata] header exceeds 50 lines ({len(lines)})")
    else:
        failures.append(f"[metadata] no {start} ... {end} header block found")

    # 3. Validations for enums
    if meta.get("stability") not in VALID_STABILITY:
        failures.append(f"[metadata] invalid @stability: {meta.get('stability')}")
    if meta.get("state-model") not in VALID_STATE_MODEL:
        failures.append(f"[metadata] invalid @state-model: {meta.get('state-model')}")
    if meta.get("side-effects") not in VALID_SIDE_EFFECTS:
        failures.append(f"[metadata] invalid @side-effects: {meta.get('side-effects')}")
    if meta.get("determinism") not in VALID_DETERMINISM:
        failures.append(f"[metadata] invalid @determinism: {meta.get('determinism')}")

    # 4. File structure order
    sections = [
        "PUBLIC INTERFACE",
        "BEHAVIOUR CONTRACT",
        "CORE IMPLEMENTATION",
        "EXAMPLE USAGE",
    ]
    positions = [text.find(s) for s in sections]
    for i, (s, p) in enumerate(zip(sections, positions)):
        if p == -1:
            failures.append(f"[structure] missing section: {s}")
        elif i > 0 and positions[i - 1] != -1 and p < positions[i - 1]:
            failures.append(f"[structure] section out of order: {s}")

    # 5. LOC limits
    loc = len([l for l in text.splitlines() if l.strip()])
    if loc > 400: # Increased to 400
        failures.append(f"[implementation] file exceeds 400 LOC ({loc})")

    # 6. Generic file name
    stem = path.stem.lower()
    if stem in {"utils", "helpers", "core", "common", "shared", "misc"}:
        failures.append(f"[capability-boundary] generic file name: {path.name}")

    # 7. module-id format
    mid = meta.get("module-id", "")
    if not re.match(r"^[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*$", mid):
        failures.append(f"[metadata] @module-id must match <domain>.<verb-noun>: {mid!r}")

    # 8. Static Consistency Checks (Heuristic)
    inputs_text = meta.get("inputs", "")
    outputs_text = meta.get("outputs", "")
    
    # Check if inputs/outputs are mentioned in the code
    impl_start = text.find("CORE IMPLEMENTATION")
    if impl_start != -1:
        impl_text = text[impl_start:]
        for line in inputs_text.splitlines():
            if ":" in line:
                param_name = line.split(":")[0].strip()
                if param_name and param_name not in impl_text:
                    failures.append(f"[consistency] declared input '{param_name}' not found in implementation")
        
        # Output check is harder, but look for types
        if ":" in outputs_text:
            out_type = outputs_text.split(":")[0].strip()
            if out_type and out_type not in text: # check whole file for type
                 failures.append(f"[consistency] declared output type '{out_type}' not found in file")

    return failures

# ─── REGISTRY ────────────────────────────────────────────────────────────────

def find_registry(start: Path) -> Path:
    """Walk up from start looking for LOCS_REGISTRY.md."""
    current = start if start.is_dir() else start.parent
    for _ in range(6):
        candidate = current / REGISTRY_FILE
        if candidate.exists():
            return candidate
        current = current.parent
    return Path.cwd() / REGISTRY_FILE

def read_registry_ids(registry: Path) -> set[str]:
    if not registry.exists():
        return set()
    text = registry.read_text(encoding="utf-8")
    ids = set()
    in_table = False
    for line in text.splitlines():
        if "| module-id |" in line:
            in_table = True
            continue
        if in_table and line.startswith("|"):
            cell = line.split("|")[1].strip()
            if cell and cell != "module-id" and "---" not in cell and "_no entries" not in cell:
                ids.add(cell)
        elif in_table and not line.startswith("|"):
            in_table = False
    return ids

def append_registry_row(registry: Path, meta: dict, file_path: str):
    text = registry.read_text(encoding="utf-8")
    
    # signature = first sentence of capability
    cap = meta.get("capability", "").split(".")[0].strip()
    
    row = (
        f"| {meta.get('module-id','')} "
        f"| {meta.get('module-name','')} "
        f"| {meta.get('domain','')} "
        f"| {meta.get('primary-capability','')} "
        f"| {cap} "
        f"| {meta.get('version','1.0.0')} "
        f"| {meta.get('stability','stable')} "
        f"| {file_path} |"
    )
    placeholder = "| _no entries yet_ |"
    today = date.today().isoformat()
    changelog_row = f"| {today} | {meta.get('module-id','')} | created |"
    changelog_placeholder = "| _no entries yet_ | | |"

    # Registry table might have different columns now
    if placeholder in text:
        text = text.replace(placeholder, row, 1)
    else:
        lines = text.splitlines()
        insert_at = None
        in_table = False
        for i, line in enumerate(lines):
            if "| module-id |" in line:
                in_table = True
            if in_table and i > 0 and not lines[i].startswith("|"):
                insert_at = i
                in_table = False
                break
        if insert_at:
            lines.insert(insert_at, row)
            text = "\n".join(lines)
        else:
            text += f"\n{row}"

    # dependency map
    deps = meta.get("dependencies", "")
    if deps and "none" not in deps.lower():
        dep_placeholder = "| _no entries yet_ | | |"
        for line in deps.splitlines():
            line = line.strip().lstrip("- ").strip()
            if line:
                dep_row = f"| {meta.get('module-id','')} | {line} | internal |"
                if dep_placeholder in text:
                    text = text.replace(dep_placeholder, dep_row, 1)
                    dep_placeholder = "REPLACED_SO_ONLY_ONCE"
                else:
                    # find dependency section and append
                    lines = text.splitlines()
                    for j, l in enumerate(lines):
                        if "## Dependency Map" in l:
                            lines.insert(j+4, dep_row)
                            break
                    text = "\n".join(lines)

    # changelog
    if changelog_placeholder in text:
        text = text.replace(changelog_placeholder, changelog_row, 1)
    else:
        text += f"\n{changelog_row}"

    registry.write_text(text, encoding="utf-8")

# ─── COMMANDS ────────────────────────────────────────────────────────────────

def cmd_new(args):
    module_id = args.module_id
    if not re.match(r"^[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*$", module_id):
        print(f"ERROR: module-id must match <domain>.<verb-noun>, got: {module_id!r}")
        sys.exit(1)

    domain, verb_noun = module_id.split(".", 1)
    module_name = "".join(w.capitalize() for w in re.split(r"[-_]", verb_noun))
    ext = args.ext or ".ts"
    start, prefix, end = COMMENT_STYLES.get(ext, ("/*", " *", " */"))
    
    out_dir = Path(args.out) if args.out else Path.cwd()
    filename = verb_noun.replace("-", "_") + ext
    out_path = out_dir / filename

    if out_path.exists() and not args.force:
        print(f"ERROR: {out_path} already exists. Use --force to overwrite.")
        sys.exit(1)

    # Idiomatic stubs
    stubs = {
        ".ts": {
            "interface": "// ─── PUBLIC INTERFACE ────────────────────────────────────────────────────────\nexport declare function {fn_name}(params: any): any;",
            "contract": "// ─── BEHAVIOUR CONTRACT ───────────────────────────────────────────────────────\n/**\n * BEHAVIOUR CONTRACT\n * - Pure function\n */",
            "impl": "// ─── CORE IMPLEMENTATION ──────────────────────────────────────────────────────\nexport function {fn_name}(params: any): any {{\n  return null;\n}}",
            "example": "// ─── EXAMPLE USAGE ───────────────────────────────────────────────────────────\n// {fn_name}(...);"
        },
        ".py": {
            "interface": "# ─── PUBLIC INTERFACE ────────────────────────────────────────────────────────\nfrom typing import Any\n\ndef {fn_name}(params: Any) -> Any:\n    \"\"\"Interface declaration.\"\"\"\n    pass",
            "contract": "# ─── BEHAVIOUR CONTRACT ───────────────────────────────────────────────────────\n# - Pure function",
            "impl": "# ─── CORE IMPLEMENTATION ──────────────────────────────────────────────────────\ndef {fn_name}(params: Any) -> Any:\n    return None",
            "example": "# ─── EXAMPLE USAGE ───────────────────────────────────────────────────────────\n# {fn_name}(...)"
        }
    }
    stub = stubs.get(ext, stubs[".ts"])
    fn_name = "".join(w if i == 0 else w.capitalize() for i, w in enumerate(re.split(r"[-_]", verb_noun)))

    content = DEFAULT_HEADER_TEMPLATE.format(
        comment_start=start,
        comment_end=end,
        prefix=prefix,
        module_id=module_id,
        module_name=module_name,
        category=args.category or domain,
        domain=domain,
        primary_capability=verb_noun.split("-")[0],
        sub_capabilities=verb_noun.replace("-", ","),
        runtime=ext.lstrip("."),
        interface_stub=stub["interface"].format(fn_name=fn_name),
        contract_stub=stub["contract"].format(fn_name=fn_name),
        implementation_stub=stub["impl"].format(fn_name=fn_name),
        example_stub=stub["example"].format(fn_name=fn_name),
    )
    out_path.write_text(content, encoding="utf-8")
    print(f"created  {out_path}")


def cmd_score(args):
    path = Path(args.file)
    if not path.exists():
        print(f"ERROR: file not found: {path}")
        sys.exit(1)
    
    text = path.read_text(encoding="utf-8")
    meta = parse_metadata(path)
    if not meta:
        print("ERROR: no LOCS metadata header found")
        sys.exit(1)
        
    # Automatic token metrics calculation
    ext = path.suffix
    start, prefix, end = COMMENT_STYLES.get(ext, ("/*", " *", " */"))
    pattern = rf"{re.escape(start)}(.*?){re.escape(end)}"
    header_match = re.search(pattern, text, re.DOTALL)
    
    if header_match:
        header_text = header_match.group(0)
        impl_text = text[header_match.end():]
        header_tokens = estimate_tokens(header_text)
        impl_tokens = estimate_tokens(impl_text)
        total_tokens = header_tokens + impl_tokens
        ratio = round(header_tokens / total_tokens, 3) if total_tokens > 0 else 0.0
        
        if args.write:
            text = re.sub(
                rf"{re.escape(prefix)}\s*@token-metrics:.*?(?=\n\s*{re.escape(prefix)}\s*@|\*/|\"\"\")",
                f"{prefix} @token-metrics:\n{prefix} - header-tokens: {header_tokens}\n{prefix} - implementation-tokens: {impl_tokens}\n{prefix} - retrieval-ratio: {ratio}",
                text, flags=re.DOTALL
            )
            # update meta for computation
            meta["token-metrics"] = f"header-tokens: {header_tokens}\nimplementation-tokens: {impl_tokens}\nretrieval-ratio: {ratio}"

    score, grade = compute_score(meta, text)
    print(f"capability-score: {score} (grade {grade})")
    
    if args.write:
        text = re.sub(r"(@capability-score[:\s]+).*", rf"\g<1>{score} (grade {grade})", text)
        path.write_text(text, encoding="utf-8")
        print(f"  updated metadata in {path.name}")


def cmd_validate(args):
    path = Path(args.file)
    if not path.exists():
        print(f"ERROR: file not found: {path}")
        sys.exit(1)
    meta = parse_metadata(path)
    failures = validate_module(path, meta)

    if failures:
        print(f"FAIL  {path.name}  ({len(failures)} issue(s))")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    else:
        score, grade = compute_score(meta, path.read_text(encoding="utf-8"))
        print(f"PASS  {path.name}  (grade {grade})")


def cmd_register(args):
    path = Path(args.file)
    if not path.exists():
        print(f"ERROR: file not found: {path}")
        sys.exit(1)

    meta = parse_metadata(path)
    if not meta.get("module-id"):
        print("ERROR: no @module-id found in metadata")
        sys.exit(1)

    if not args.skip_validate:
        failures = validate_module(path, meta)
        if failures:
            print(f"FAIL  validation failed ({len(failures)} issue(s))")
            for f in failures: print(f"  - {f}")
            sys.exit(1)

    registry = find_registry(path)
    existing_ids = read_registry_ids(registry)
    mid = meta["module-id"]
    if mid in existing_ids:
        print(f"ERROR: collision — {mid!r} exists")
        sys.exit(1)

    # Dependency integrity check
    deps = meta.get("dependencies", "")
    if deps and "none" not in deps.lower():
        for line in deps.splitlines():
            line = line.strip().lstrip("- ").strip()
            if line and "(internal)" in line:
                dep_id = line.split("(")[0].strip()
                if dep_id not in existing_ids:
                    print(f"ERROR: broken dependency — {dep_id!r} not in registry")
                    sys.exit(1)

    append_registry_row(registry, meta, str(path.relative_to(registry.parent)))
    print(f"registered  {mid}")


def cmd_status(args):
    registry = Path(args.registry) if args.registry else find_registry(Path.cwd())
    if not registry.exists():
        print("no registry found")
        return

    text = registry.read_text(encoding="utf-8")
    print(f"LOCS Registry — {registry}")
    # Simple status print
    in_table = False
    for line in text.splitlines():
        if "| module-id |" in line:
            in_table = True
            print(line)
            continue
        if in_table and line.startswith("|"):
            print(line)
        elif in_table:
            break

def cmd_bootstrap(args):
    """Context-aware bootstrap output."""
    registry = find_registry(Path.cwd())
    if not registry.exists():
        print("ERROR: registry not found")
        sys.exit(1)

    text = registry.read_text(encoding="utf-8")
    modules = []
    in_table = False
    for line in text.splitlines():
        if "| module-id |" in line:
            in_table = True
            continue
        if in_table and line.startswith("|"):
            if "---" in line:
                continue
            cells = [c.strip() for c in line.split("|")[1:-1]]
            if cells and "_no entries" not in cells[0]:
                # module-id, name, domain, primary-cap, signature, version, stability, file
                modules.append({
                    "id": cells[0],
                    "name": cells[1],
                    "domain": cells[2],
                    "category": cells[3], # if columns changed
                    "primary": cells[3],
                    "sig": cells[4],
                })
        elif in_table:
            in_table = False

    if args.category:
        modules = [m for m in modules if m["category"] == args.category]
    if args.domain:
        modules = [m for m in modules if m["domain"] == args.domain]

    print("--- LOCS BOOTSTRAP (Condensed Signatures) ---")
    for m in modules:
        print(f"@module: {m['id']} | {m['primary']} | {m['sig']}")
    print("---------------------------------------------")

# ─── ENTRY POINT ─────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(prog="locs", description="LOCS CLI v1.2")
    sub = parser.add_subparsers(dest="command", required=True)

    p_new = sub.add_parser("new", help="Scaffold module")
    p_new.add_argument("module_id")
    p_new.add_argument("--out")
    p_new.add_argument("--ext")
    p_new.add_argument("--category")
    p_new.add_argument("--force", action="store_true")

    p_score = sub.add_parser("score", help="Score module")
    p_score.add_argument("file")
    p_score.add_argument("--write", action="store_true")

    p_val = sub.add_parser("validate", help="Validate module")
    p_val.add_argument("file")

    p_reg = sub.add_parser("register", help="Register module")
    p_reg.add_argument("file")
    p_reg.add_argument("--skip-validate", action="store_true")

    p_status = sub.add_parser("status", help="Registry status")
    p_status.add_argument("--registry")

    p_boot = sub.add_parser("bootstrap", help="Context-aware bootstrap")
    p_boot.add_argument("--category")
    p_boot.add_argument("--domain")

    args = parser.parse_args()
    dispatch = {
        "new": cmd_new, "score": cmd_score, "validate": cmd_validate,
        "register": cmd_register, "status": cmd_status, "bootstrap": cmd_bootstrap
    }
    dispatch[args.command](args)


if __name__ == "__main__":
    main()
