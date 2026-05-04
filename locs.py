#!/usr/bin/env python3
"""
LOCS CLI (v1.4)
Commands: new | score | validate | register | status | bootstrap
Optional tokenizer and AST backends with deterministic fallbacks.
"""

import argparse
import ast
import re
import sys
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import Any

LOCAL_REGISTRY_FILE = "LOCS_REGISTRY.md"
SHARED_REGISTRY_FILE = "LOCS_GRAND_REGISTRY.md"
MAX_DEPENDENCY_DEPTH = 5
MAX_HEADER_LINES = 80
MINIMUM_PASSING_GRADE = {"A", "B", "C"}
TREE_SITTER_JS_LANG = "javascript"
TREE_SITTER_TS_LANG = "typescript"

REQUIRED_FIELDS = [
    "locs-version", "module-id", "module-name", "category", "domain",
    "primary-capability", "sub-capabilities", "version", "stability",
    "state-model", "side-effects", "determinism", "complexity",
    "dependency-depth", "runtime", "framework-agnostic", "capability",
    "inputs", "outputs", "preconditions", "postconditions",
    "use-when", "avoid-when", "token-metrics", "capability-score",
    "registry-entry-required",
]

OPTIONAL_FIELDS = ["summary", "module", "usage-metrics"]

VALID_STABILITY = {"stable", "experimental", "deprecated"}
VALID_STATE_MODEL = {"stateless", "explicit-state", "event-driven", "async-io", "external-boundary"}
VALID_SIDE_EFFECTS = {"none", "explicit", "high"}
VALID_DETERMINISM = {"deterministic", "probabilistic", "async-nondeterministic"}

COMMENT_STYLES = {
    ".ts": ("/**", " *", " */"),
    ".js": ("/**", " *", " */"),
    ".py": ('"""', "", '"""'),
    ".go": ("/*", " *", " */"),
    ".rs": ("/*", " *", " */"),
    ".java": ("/**", " *", " */"),
    ".c": ("/*", " *", " */"),
    ".cpp": ("/*", " *", " */"),
}

LOCAL_REGISTRY_TEMPLATE = """# LOCS Project Registry (v1.4)

<!-- LOCS-REGISTRY-SCHEMA
format: markdown-table
parser: locs-registry-v1.4
scope: local
automated-writes: true
row-format: | module-id | module-name | category | domain | primary-capability | signature | version | stability | file-path | capability-score |
placeholder-row: | _no entries yet_ | | | | | | | | | |
dependency-row-format: | module-id | depends-on | type |
changelog-row-format: | YYYY-MM-DD | module-id | change |
-->

---

## Registry Table

| module-id | module-name | category | domain | primary-capability | signature | version | stability | file-path | capability-score |
|---|---|---|---|---|---|---|---|---|---|
| _no entries yet_ | | | | | | | | | |

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
"""

DEFAULT_HEADER_TEMPLATE = """\
{comment_start}
{prefix} @locs-version: 1.2
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
{prefix} - tokenizer-backend: heuristic
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


@dataclass
class AstValidationResult:
    backend: str
    declared_inputs_found: set[str] = field(default_factory=set)
    declared_output_found: bool | None = None
    side_effect_hits: list[str] = field(default_factory=list)
    parse_error: str = ""


def estimate_tokens(text: str) -> int:
    words = text.split()
    return int(max(len(words) * 1.3, len(text) / 4))


def _count_tokens_tiktoken(text: str, tokenizer: str, model: str) -> tuple[int, str] | None:
    try:
        import tiktoken  # type: ignore
    except ImportError:
        return None

    try:
        if model:
            encoding = tiktoken.encoding_for_model(model)
            return len(encoding.encode(text)), f"tiktoken:{model}"
        if tokenizer not in {"auto", "tiktoken"}:
            encoding = tiktoken.get_encoding(tokenizer)
            return len(encoding.encode(text)), f"tiktoken:{tokenizer}"
        encoding = tiktoken.get_encoding("cl100k_base")
        return len(encoding.encode(text)), "tiktoken:cl100k_base"
    except Exception:
        return None


def _count_tokens_transformers(text: str, model: str) -> tuple[int, str] | None:
    if not model:
        return None
    try:
        from transformers import AutoTokenizer  # type: ignore
    except ImportError:
        return None

    try:
        tokenizer = AutoTokenizer.from_pretrained(model)
        return len(tokenizer.encode(text, add_special_tokens=False)), f"transformers:{model}"
    except Exception:
        return None


def _count_tokens_sentencepiece(text: str, resource: str) -> tuple[int, str] | None:
    if not resource:
        return None
    resource_path = Path(resource)
    if not resource_path.exists():
        return None
    try:
        import sentencepiece as spm  # type: ignore
    except ImportError:
        return None

    try:
        processor = spm.SentencePieceProcessor(model_file=str(resource_path))
        return len(processor.encode(text, out_type=int)), f"sentencepiece:{resource_path.name}"
    except Exception:
        return None


def count_tokens(
    text: str,
    tokenizer: str = "auto",
    model: str = "",
    tokenizer_resource: str = "",
) -> tuple[int, str]:
    if tokenizer == "heuristic":
        return estimate_tokens(text), "heuristic"

    strategies = []
    if tokenizer == "tiktoken":
        strategies = [lambda: _count_tokens_tiktoken(text, tokenizer, model)]
    elif tokenizer == "transformers":
        strategies = [lambda: _count_tokens_transformers(text, model)]
    elif tokenizer == "sentencepiece":
        strategies = [lambda: _count_tokens_sentencepiece(text, tokenizer_resource or model)]
    else:
        strategies = [
            lambda: _count_tokens_tiktoken(text, tokenizer, model),
            lambda: _count_tokens_transformers(text, model),
            lambda: _count_tokens_sentencepiece(text, tokenizer_resource or model),
        ]

    for strategy in strategies:
        result = strategy()
        if result is not None:
            return result
    return estimate_tokens(text), "heuristic"


def parse_metric_value(block: str, key: str) -> str:
    for line in block.splitlines():
        cleaned = line.strip().lstrip("- ").strip()
        if cleaned.startswith(f"{key}:"):
            return cleaned.split(":", 1)[1].strip()
    return ""


def parse_capability_score(meta: dict[str, str]) -> tuple[float | None, str | None]:
    raw = meta.get("capability-score", "").strip()
    if not raw or raw == "auto":
        return None, None
    match = re.search(r"([0-9]*\.?[0-9]+)\s*\(grade\s+([A-F])\)", raw, re.IGNORECASE)
    if not match:
        return None, None
    return float(match.group(1)), match.group(2).upper()


def parse_metadata(path: Path) -> dict[str, str]:
    text = path.read_text(encoding="utf-8")
    meta: dict[str, str] = {}

    start, prefix, end = COMMENT_STYLES.get(path.suffix, ("/*", " *", " */"))
    block_match = re.search(rf"{re.escape(start)}(.*?){re.escape(end)}", text, re.DOTALL)
    if not block_match:
        return meta

    block = block_match.group(1)
    for line in block.splitlines():
        line = line.strip()
        if prefix and line.startswith(prefix.strip()):
            line = line[len(prefix.strip()):].strip()
        match = re.match(r"@([\w-]+):\s*(.*)", line)
        if match and match.group(1) not in meta:
            meta[match.group(1)] = match.group(2).strip()

    for field in [
        "capability", "inputs", "outputs", "preconditions", "postconditions",
        "use-when", "avoid-when", "token-metrics", "usage-metrics",
        "summary", "dependencies", "compatibility",
    ]:
        prefix_pattern = re.escape(prefix.strip())
        block_pattern = rf"@{field}:\s*\n(.*?)(?=\n\s*{prefix_pattern}\s*@|\*/|\"\"\")"
        match = re.search(block_pattern, block, re.DOTALL)
        if not match:
            continue
        cleaned_lines = []
        for raw_line in match.group(1).splitlines():
            raw_line = raw_line.strip()
            if prefix and raw_line.startswith(prefix.strip()):
                raw_line = raw_line[len(prefix.strip()):].strip()
            cleaned_lines.append(raw_line)
        meta[field] = "\n".join(cleaned_lines).strip()

    return meta


def replace_metadata_block(text: str, path: Path, field: str, new_block: str) -> str:
    _start, prefix, _end = COMMENT_STYLES.get(path.suffix, ("/*", " *", " */"))
    next_field_pattern = re.escape(prefix.strip()) if prefix.strip() else ""
    if next_field_pattern:
        pattern = rf"(@{field}:\s*\n)(.*?)(?=\n\s*{next_field_pattern}\s*@|\n\s*@|\*/|\"\"\")"
    else:
        pattern = rf"(@{field}:\s*\n)(.*?)(?=\n\s*@|\*/|\"\"\")"
    first_line = f"{prefix} @{field}:" if prefix else f"@{field}:"
    block_lines = [first_line]
    block_lines.extend(new_block.splitlines())
    replacement = "\n".join(block_lines)
    return re.sub(pattern, replacement, text, flags=re.DOTALL)


def _pascal_word_count(name: str) -> int:
    return len(re.findall(r"[A-Z][a-z0-9]*", name))


def _capability_word_count(capability: str) -> int:
    return len(capability.split())


def compute_score(meta: dict[str, str]) -> tuple[float, str]:
    subs = [item.strip() for item in meta.get("sub-capabilities", "").split(",") if item.strip()]
    sub_score = min(len(subs), 5) / 5

    primary_score = 1.0 if meta.get("primary-capability", "").strip() else 0.0
    capability = meta.get("capability", "")
    capability_words = _capability_word_count(capability)
    if 5 <= capability_words <= 12:
        capability_score = 1.0
    elif capability_words < 5:
        capability_score = capability_words / 5 if capability_words else 0.0
    else:
        capability_score = 12 / capability_words

    module_name = meta.get("module-name", "")
    name_words = _pascal_word_count(module_name)
    if 2 <= name_words <= 4:
        name_score = 1.0
    elif name_words < 2:
        name_score = name_words / 2 if name_words else 0.0
    else:
        name_score = 4 / name_words

    retrieval = (sub_score + primary_score + capability_score + name_score) / 4

    det_map = {"deterministic": 1.0, "probabilistic": 0.5, "async-nondeterministic": 0.2}
    side_map = {"none": 1.0, "explicit": 0.6, "high": 0.2}
    state_map = {
        "stateless": 1.0,
        "explicit-state": 0.8,
        "event-driven": 0.6,
        "async-io": 0.4,
        "external-boundary": 0.2,
    }
    determinism = (
        det_map.get(meta.get("determinism", ""), 0.0)
        + side_map.get(meta.get("side-effects", ""), 0.0)
        + state_map.get(meta.get("state-model", ""), 0.0)
    ) / 3

    ratio = 0.1
    ratio_text = parse_metric_value(meta.get("token-metrics", ""), "retrieval-ratio")
    if ratio_text:
        try:
            ratio = float(ratio_text)
        except ValueError:
            ratio = 0.1
    token_efficiency = min(1.0, 1.0 - ratio)

    complexity = meta.get("complexity", "O(n)").upper()
    performance = {
        "O(1)": 1.0,
        "O(LOG N)": 0.9,
        "O(N)": 0.8,
        "O(N LOG N)": 0.6,
        "O(N^2)": 0.4,
        "O(2^N)": 0.1,
        "O(N!)": 0.0,
    }.get(complexity, 0.5)

    present = sum(1 for field_name in REQUIRED_FIELDS if meta.get(field_name, "").strip())
    structural = min(
        1.0,
        (present / len(REQUIRED_FIELDS))
        + sum(0.5 / len(REQUIRED_FIELDS) for field_name in OPTIONAL_FIELDS if meta.get(field_name, "").strip()),
    )

    try:
        depth = int(meta.get("dependency-depth", "0"))
    except ValueError:
        depth = 0
    depth_score = 1 / (1 + depth)
    dependents_text = parse_metric_value(meta.get("usage-metrics", ""), "dependents")
    dependents = int(dependents_text) if dependents_text.isdigit() else 0
    usage_bonus = min(0.2, dependents * 0.01)
    isolation = min(
        1.0,
        (depth_score + (1.0 if meta.get("framework-agnostic") == "true" else 0.0)) / 2 + usage_bonus,
    )

    total = round(
        0.25 * retrieval
        + 0.20 * determinism
        + 0.20 * token_efficiency
        + 0.15 * performance
        + 0.15 * structural
        + 0.05 * isolation,
        3,
    )

    if total >= 0.90:
        grade = "A"
    elif total >= 0.75:
        grade = "B"
    elif total >= 0.60:
        grade = "C"
    elif total >= 0.40:
        grade = "D"
    else:
        grade = "F"
    return total, grade


def extract_internal_dependencies(meta: dict[str, str]) -> list[str]:
    dependencies = []
    for line in meta.get("dependencies", "").splitlines():
        cleaned = line.strip().lstrip("- ").strip()
        if not cleaned or cleaned.lower() == "none":
            continue
        dependency_id = cleaned.split("(")[0].strip()
        kind_match = re.search(r"\(([^)]+)\)", cleaned)
        kind = kind_match.group(1).strip().lower() if kind_match else "internal"
        if kind == "internal" and dependency_id:
            dependencies.append(dependency_id)
    return dependencies


def extract_declared_inputs(meta: dict[str, str]) -> list[str]:
    declared = []
    for line in meta.get("inputs", "").splitlines():
        if ":" in line:
            declared.append(line.split(":", 1)[0].strip())
    return [item for item in declared if item]


def _normalize_type_label(value: str) -> str:
    return value.strip().replace(" ", "").lower()


def _python_type_name(node: ast.AST | None) -> str:
    if node is None:
        return ""
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        return node.attr
    if isinstance(node, ast.Subscript):
        return _python_type_name(node.value)
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    if hasattr(ast, "unparse"):
        try:
            return ast.unparse(node)
        except Exception:
            return ""
    return ""


class PythonAstInspector(ast.NodeVisitor):
    def __init__(self):
        self.signature_params: set[str] = set()
        self.return_annotations: set[str] = set()
        self.side_effect_hits: list[str] = []

    def visit_FunctionDef(self, node: ast.FunctionDef):
        self._capture_function(node)
        self.generic_visit(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef):
        self._capture_function(node)
        self.generic_visit(node)

    def _capture_function(self, node: ast.FunctionDef | ast.AsyncFunctionDef):
        args = list(node.args.posonlyargs) + list(node.args.args) + list(node.args.kwonlyargs)
        for arg in args:
            self.signature_params.add(arg.arg)
        if node.args.vararg:
            self.signature_params.add(node.args.vararg.arg)
        if node.args.kwarg:
            self.signature_params.add(node.args.kwarg.arg)
        if node.returns is not None:
            self.return_annotations.add(_normalize_type_label(_python_type_name(node.returns)))

    def visit_Call(self, node: ast.Call):
        callee = _python_call_name(node.func)
        if callee in {
            "print", "open", "requests.get", "requests.post", "requests.put", "requests.delete",
            "subprocess.run", "subprocess.call", "subprocess.Popen",
            "pathlib.Path.write_text", "pathlib.Path.write_bytes", "pathlib.Path.mkdir",
            "pathlib.Path.unlink", "os.remove", "os.rename", "os.makedirs",
        }:
            self.side_effect_hits.append(callee)
        self.generic_visit(node)


def _python_call_name(node: ast.AST) -> str:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        left = _python_call_name(node.value)
        return f"{left}.{node.attr}" if left else node.attr
    return ""


def inspect_python_ast(path: Path, text: str, declared_inputs: list[str], declared_output: str) -> AstValidationResult:
    try:
        tree = ast.parse(text, filename=str(path))
    except SyntaxError as exc:
        return AstValidationResult(backend="python-ast", parse_error=str(exc))

    inspector = PythonAstInspector()
    inspector.visit(tree)
    normalized_outputs = {_normalize_type_label(item) for item in inspector.return_annotations if item}
    return AstValidationResult(
        backend="python-ast",
        declared_inputs_found={item for item in declared_inputs if item in inspector.signature_params},
        declared_output_found=_normalize_type_label(declared_output) in normalized_outputs if declared_output else None,
        side_effect_hits=sorted(set(inspector.side_effect_hits)),
    )


def _load_tree_sitter_language(path: Path):
    language_name = TREE_SITTER_TS_LANG if path.suffix == ".ts" else TREE_SITTER_JS_LANG
    try:
        from tree_sitter_languages import get_language  # type: ignore

        return get_language(language_name), f"tree-sitter-languages:{language_name}"
    except ImportError:
        pass
    except Exception:
        return None, ""

    try:
        import tree_sitter_javascript as ts_javascript  # type: ignore
        import tree_sitter_typescript as ts_typescript  # type: ignore
        from tree_sitter import Language  # type: ignore
    except ImportError:
        return None, ""

    try:
        if path.suffix == ".ts":
            capsule = ts_typescript.language_typescript()
            return Language(capsule), f"tree-sitter:{language_name}"
        capsule = ts_javascript.language()
        return Language(capsule), f"tree-sitter:{language_name}"
    except Exception:
        return None, ""


def inspect_js_ts_ast(path: Path, text: str, declared_inputs: list[str], declared_output: str) -> AstValidationResult:
    try:
        from tree_sitter import Parser  # type: ignore
    except ImportError:
        return AstValidationResult(backend="regex-fallback")

    language, backend = _load_tree_sitter_language(path)
    if language is None:
        return AstValidationResult(backend="regex-fallback")

    try:
        parser = Parser()
        parser.language = language
        tree = parser.parse(text.encode("utf-8"))
    except Exception as exc:
        return AstValidationResult(backend=backend or "tree-sitter", parse_error=str(exc))

    params_found: set[str] = set()
    side_effect_hits: list[str] = []
    output_found = None if not declared_output else False
    source_bytes = text.encode("utf-8")
    call_patterns = {
        "console": {"console.log", "console.error", "console.warn"},
        "fetch": {"fetch"},
        "axios": {"axios", "axios.get", "axios.post", "axios.put", "axios.delete"},
        "fs": {"fs.writeFile", "fs.writeFileSync", "fs.mkdir", "fs.unlink", "fs.rm"},
    }

    def node_text(node) -> str:
        return source_bytes[node.start_byte:node.end_byte].decode("utf-8", errors="ignore")

    def walk(node):
        nonlocal output_found
        if node.type in {"formal_parameters", "required_parameter", "optional_parameter"}:
            for child in node.children:
                if child.type in {"identifier", "object_pattern", "array_pattern"}:
                    candidate = node_text(child).strip()
                    if candidate in declared_inputs:
                        params_found.add(candidate)

        if declared_output and node.type in {"type_annotation", "predefined_type", "type_identifier"}:
            normalized = _normalize_type_label(node_text(node))
            if _normalize_type_label(declared_output) and _normalize_type_label(declared_output) in normalized:
                output_found = True

        if node.type == "call_expression" and node.children:
            callee = node_text(node.children[0]).strip()
            for label, options in call_patterns.items():
                if callee in options:
                    side_effect_hits.append(callee)
                    break

        for child in node.children:
            walk(child)

    walk(tree.root_node)
    return AstValidationResult(
        backend=backend or "tree-sitter",
        declared_inputs_found=params_found,
        declared_output_found=output_found,
        side_effect_hits=sorted(set(side_effect_hits)),
    )


def inspect_ast(path: Path, text: str, declared_inputs: list[str], declared_output: str) -> AstValidationResult:
    if path.suffix == ".py":
        return inspect_python_ast(path, text, declared_inputs, declared_output)
    if path.suffix in {".js", ".ts"}:
        return inspect_js_ts_ast(path, text, declared_inputs, declared_output)
    return AstValidationResult(backend="regex-fallback")


def extract_implementation_signature_region(path: Path, text: str) -> str:
    impl_start = text.find("CORE IMPLEMENTATION")
    if impl_start == -1:
        return ""
    impl_text = text[impl_start:]
    if path.suffix == ".py":
        return "\n".join(re.findall(r"def\s+\w+\s*\((.*?)\)\s*(?:->\s*[^:]+)?\s*:", impl_text, re.DOTALL))
    if path.suffix in {".ts", ".js"}:
        return "\n".join(re.findall(r"(?:export\s+)?function\s+\w+\s*\((.*?)\)", impl_text, re.DOTALL))
    return impl_text[:500]


def detect_side_effect_patterns(path: Path, impl_text: str) -> list[str]:
    patterns = {
        ".py": [
            r"\bprint\s*\(",
            r"\bopen\s*\(",
            r"\brequests\.",
            r"\bsubprocess\.",
            r"\bos\.(remove|rename|makedirs)\b",
            r"\bPath\([^)]*\)\.(write_text|write_bytes|mkdir|unlink)\b",
        ],
        ".ts": [
            r"\bconsole\.",
            r"\bfetch\s*\(",
            r"\baxios\.",
            r"\bfs\.(writeFile|writeFileSync|mkdir|unlink|rm)\b",
        ],
        ".js": [
            r"\bconsole\.",
            r"\bfetch\s*\(",
            r"\baxios\.",
            r"\bfs\.(writeFile|writeFileSync|mkdir|unlink|rm)\b",
        ],
        ".go": [
            r"\bfmt\.Print",
            r"\bos\.(WriteFile|Mkdir|Remove)\b",
            r"\bhttp\.(Get|Post)\b",
        ],
    }
    return [pattern for pattern in patterns.get(path.suffix, []) if re.search(pattern, impl_text)]


def find_registry_file(start: Path, filename: str) -> Path | None:
    current = start if start.is_dir() else start.parent
    for _ in range(6):
        candidate = current / filename
        if candidate.exists():
            return candidate
        current = current.parent
    return None


def resolve_registry(start: Path, scope: str = "auto", explicit: str | None = None) -> Path:
    if explicit:
        return Path(explicit)
    if scope == "shared":
        return find_registry_file(start, SHARED_REGISTRY_FILE) or (Path.cwd() / SHARED_REGISTRY_FILE)
    if scope == "local":
        return find_registry_file(start, LOCAL_REGISTRY_FILE) or (Path.cwd() / LOCAL_REGISTRY_FILE)
    local = find_registry_file(start, LOCAL_REGISTRY_FILE)
    if local:
        return local
    shared = find_registry_file(start, SHARED_REGISTRY_FILE)
    if shared:
        return shared
    return Path.cwd() / LOCAL_REGISTRY_FILE


def ensure_registry_exists(registry: Path, scope: str):
    if registry.exists():
        return
    if registry.name == LOCAL_REGISTRY_FILE or scope == "local":
        registry.write_text(LOCAL_REGISTRY_TEMPLATE, encoding="utf-8")
        return
    raise FileNotFoundError(f"registry not found: {registry}")


def extract_registry_data(registry: Path) -> tuple[dict[str, dict[str, str]], dict[str, list[str]]]:
    if not registry.exists():
        return {}, {}

    text = registry.read_text(encoding="utf-8")
    modules: dict[str, dict[str, str]] = {}
    graph: dict[str, list[str]] = {}
    section = ""
    for line in text.splitlines():
        if line.startswith("## "):
            section = line.strip()
            continue
        if not line.startswith("|") or "---" in line or "_no entries yet_" in line or "| module-id |" in line:
            continue

        cells = [cell.strip() for cell in line.split("|")[1:-1]]
        if section == "## Registry Table":
            if len(cells) >= 10:
                modules[cells[0]] = {
                    "module-id": cells[0],
                    "module-name": cells[1],
                    "category": cells[2],
                    "domain": cells[3],
                    "primary-capability": cells[4],
                    "signature": cells[5],
                    "version": cells[6],
                    "stability": cells[7],
                    "file-path": cells[8],
                    "capability-score": cells[9],
                }
            elif len(cells) >= 8:
                modules[cells[0]] = {
                    "module-id": cells[0],
                    "module-name": cells[1],
                    "category": "",
                    "domain": cells[2],
                    "primary-capability": cells[3],
                    "signature": cells[4],
                    "version": cells[5],
                    "stability": cells[6],
                    "file-path": cells[7],
                    "capability-score": "",
                }
        elif section in {"## Dependency Map", "## Dependency Map (Cross-Project)"} and len(cells) >= 3:
            if cells[2].lower() == "internal":
                graph.setdefault(cells[0], []).append(cells[1])
    return modules, graph


def read_registry_ids(registry: Path) -> set[str]:
    modules, _ = extract_registry_data(registry)
    return set(modules.keys())


def compute_dependency_depth(module_id: str, graph: dict[str, list[str]], seen: set[str] | None = None) -> int:
    seen = seen or set()
    if module_id in seen:
        return MAX_DEPENDENCY_DEPTH + 1
    dependencies = graph.get(module_id, [])
    if not dependencies:
        return 0
    next_seen = set(seen)
    next_seen.add(module_id)
    return 1 + max(compute_dependency_depth(dep, graph, next_seen) for dep in dependencies)


def find_cycle(module_id: str, graph: dict[str, list[str]], stack: list[str] | None = None) -> list[str]:
    stack = stack or []
    if module_id in stack:
        return stack[stack.index(module_id):] + [module_id]
    next_stack = stack + [module_id]
    for dependency in graph.get(module_id, []):
        cycle = find_cycle(dependency, graph, next_stack)
        if cycle:
            return cycle
    return []


def append_registry_row(registry: Path, meta: dict[str, str], file_path: str):
    lines = registry.read_text(encoding="utf-8").splitlines()
    signature = meta.get("capability", "").split(".")[0].strip()
    row = (
        f"| {meta.get('module-id', '')} "
        f"| {meta.get('module-name', '')} "
        f"| {meta.get('category', '')} "
        f"| {meta.get('domain', '')} "
        f"| {meta.get('primary-capability', '')} "
        f"| {signature} "
        f"| {meta.get('version', '1.0.0')} "
        f"| {meta.get('stability', 'stable')} "
        f"| {file_path} "
        f"| {meta.get('capability-score', '')} |"
    )
    changelog_row = f"| {date.today().isoformat()} | {meta.get('module-id', '')} | created |"
    dependencies = []
    for dep_line in meta.get("dependencies", "").splitlines():
        cleaned = dep_line.strip().lstrip("- ").strip()
        if cleaned and cleaned.lower() != "none":
            dependencies.append(f"| {meta.get('module-id', '')} | {cleaned} | internal |")

    def insert_table_row(section_name: str, new_row: str):
        nonlocal lines
        section_index = next((i for i, line in enumerate(lines) if line.strip() == section_name), None)
        if section_index is None:
            lines.append(new_row)
            return
        table_start = None
        for index in range(section_index + 1, len(lines)):
            if lines[index].startswith("|"):
                table_start = index
                break
        if table_start is None:
            lines.append(new_row)
            return
        placeholder_index = None
        insert_at = None
        for index in range(table_start + 2, len(lines)):
            current = lines[index]
            if not current.startswith("|"):
                insert_at = index
                break
            if "_no entries yet_" in current:
                placeholder_index = index
                break
        if placeholder_index is not None:
            lines[placeholder_index] = new_row
        elif insert_at is not None:
            lines.insert(insert_at, new_row)
        else:
            lines.append(new_row)

    dependency_section = "## Dependency Map (Cross-Project)" if registry.name == SHARED_REGISTRY_FILE else "## Dependency Map"
    changelog_section = "## Global Changelog" if registry.name == SHARED_REGISTRY_FILE else "## Changelog"

    insert_table_row("## Registry Table", row)
    for dep_row in dependencies:
        insert_table_row(dependency_section, dep_row)
    insert_table_row(changelog_section, changelog_row)

    registry.write_text("\n".join(lines) + "\n", encoding="utf-8")


def validate_module(path: Path, meta: dict[str, str], registry: Path | None = None) -> list[str]:
    failures = []
    text = path.read_text(encoding="utf-8")

    for field_name in REQUIRED_FIELDS:
        if not meta.get(field_name, "").strip():
            failures.append(f"[metadata] missing or empty: @{field_name}")

    capability_score, grade = parse_capability_score(meta)
    if capability_score is None or grade is None:
        failures.append("[metadata] @capability-score must be populated via `locs score --write`")
    elif grade not in MINIMUM_PASSING_GRADE:
        failures.append(f"[metadata] capability grade must be >= C, got {grade}")

    header_start, _prefix, header_end = COMMENT_STYLES.get(path.suffix, ("/*", " *", " */"))
    block_match = re.search(rf"{re.escape(header_start)}(.*?){re.escape(header_end)}", text, re.DOTALL)
    if block_match:
        header_lines = len([line for line in block_match.group(1).splitlines() if line.strip()])
        if header_lines > MAX_HEADER_LINES:
            failures.append(f"[metadata] header exceeds {MAX_HEADER_LINES} non-empty lines ({header_lines})")
    else:
        failures.append(f"[metadata] no {header_start} ... {header_end} header block found")

    if meta.get("stability") not in VALID_STABILITY:
        failures.append(f"[metadata] invalid @stability: {meta.get('stability')}")
    if meta.get("state-model") not in VALID_STATE_MODEL:
        failures.append(f"[metadata] invalid @state-model: {meta.get('state-model')}")
    if meta.get("side-effects") not in VALID_SIDE_EFFECTS:
        failures.append(f"[metadata] invalid @side-effects: {meta.get('side-effects')}")
    if meta.get("determinism") not in VALID_DETERMINISM:
        failures.append(f"[metadata] invalid @determinism: {meta.get('determinism')}")

    if re.search(r"^\s*(?:\*|#)?\s*@[\w-]+\s+\S+", block_match.group(1) if block_match else "", re.MULTILINE):
        failures.append("[metadata] strict @key: value syntax violated")

    for section in ["PUBLIC INTERFACE", "BEHAVIOUR CONTRACT", "CORE IMPLEMENTATION", "EXAMPLE USAGE"]:
        if text.find(section) == -1:
            failures.append(f"[structure] missing section: {section}")
    section_positions = [text.find(section) for section in ["PUBLIC INTERFACE", "BEHAVIOUR CONTRACT", "CORE IMPLEMENTATION", "EXAMPLE USAGE"]]
    for index in range(1, len(section_positions)):
        if section_positions[index - 1] != -1 and section_positions[index] != -1 and section_positions[index] < section_positions[index - 1]:
            failures.append("[structure] section ordering is invalid")
            break

    non_empty_lines = len([line for line in text.splitlines() if line.strip()])
    if non_empty_lines > 400:
        failures.append(f"[implementation] file exceeds 400 LOC ({non_empty_lines})")

    if path.stem.lower() in {"utils", "helpers", "core", "common", "shared", "misc"}:
        failures.append(f"[capability-boundary] generic file name: {path.name}")

    module_id = meta.get("module-id", "")
    if not re.match(r"^[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*$", module_id):
        failures.append(f"[metadata] @module-id must match <domain>.<verb-noun>: {module_id!r}")

    impl_start = text.find("CORE IMPLEMENTATION")
    if impl_start != -1:
        impl_text = text[impl_start:]
        declared_inputs = extract_declared_inputs(meta)
        declared_output = meta.get("outputs", "").split(":", 1)[0].strip()
        ast_result = inspect_ast(path, text, declared_inputs, declared_output)

        if ast_result.parse_error:
            failures.append(f"[analysis] {ast_result.backend} parse failed: {ast_result.parse_error}")

        if ast_result.backend == "regex-fallback":
            signature_region = extract_implementation_signature_region(path, text)
            for declared_input in declared_inputs:
                if declared_input not in signature_region:
                    failures.append(f"[consistency] declared input '{declared_input}' not found in implementation signatures")
            if declared_output and declared_output not in text:
                failures.append(f"[consistency] declared output type '{declared_output}' not found in file")
            side_effect_hits = detect_side_effect_patterns(path, impl_text)
        else:
            for declared_input in declared_inputs:
                if declared_input not in ast_result.declared_inputs_found:
                    failures.append(f"[consistency] declared input '{declared_input}' not found in AST signatures")
            if declared_output and ast_result.declared_output_found is False:
                failures.append(f"[consistency] declared output type '{declared_output}' not found in AST signatures")
            side_effect_hits = ast_result.side_effect_hits

        if meta.get("side-effects") == "none" and side_effect_hits:
            failures.append(f"[consistency] @side-effects is none but analysis found side-effect-like calls: {', '.join(side_effect_hits)}")

    for dependency_id in extract_internal_dependencies(meta):
        if not re.match(r"^[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*$", dependency_id):
            failures.append(f"[dependencies] invalid module-id in @dependencies: {dependency_id!r}")

    if registry and registry.exists():
        existing_modules, graph = extract_registry_data(registry)
        declared_dependencies = extract_internal_dependencies(meta)
        for dependency_id in declared_dependencies:
            if dependency_id not in existing_modules:
                failures.append(f"[dependencies] internal dependency not found in registry: {dependency_id}")

        trial_graph = dict(graph)
        trial_graph[module_id] = declared_dependencies
        cycle = find_cycle(module_id, trial_graph)
        if cycle:
            failures.append(f"[dependencies] circular dependency detected: {' -> '.join(cycle)}")

        computed_depth = compute_dependency_depth(module_id, trial_graph)
        declared_depth = meta.get("dependency-depth", "0").strip()
        if declared_depth.isdigit() and int(declared_depth) != computed_depth:
            failures.append(f"[dependencies] @dependency-depth={declared_depth} does not match computed depth {computed_depth}")
        if computed_depth > MAX_DEPENDENCY_DEPTH:
            failures.append(f"[dependencies] dependency depth {computed_depth} exceeds cap {MAX_DEPENDENCY_DEPTH}")

    return failures


def cmd_new(args):
    module_id = args.module_id
    if not re.match(r"^[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*$", module_id):
        print(f"ERROR: module-id must match <domain>.<verb-noun>, got: {module_id!r}")
        sys.exit(1)

    domain, verb_noun = module_id.split(".", 1)
    module_name = "".join(word.capitalize() for word in re.split(r"[-_]", verb_noun))
    ext = args.ext or ".ts"
    start, prefix, end = COMMENT_STYLES.get(ext, ("/*", " *", " */"))
    out_dir = Path(args.out) if args.out else Path.cwd()
    filename = verb_noun.replace("-", "_") + ext
    out_path = out_dir / filename

    if out_path.exists() and not args.force:
        print(f"ERROR: {out_path} already exists. Use --force to overwrite.")
        sys.exit(1)

    stubs = {
        ".ts": {
            "interface": "// PUBLIC INTERFACE\nexport declare function {fn_name}(params: any): any;",
            "contract": "// BEHAVIOUR CONTRACT\n/**\n * - Pure function\n */",
            "impl": "// CORE IMPLEMENTATION\nexport function {fn_name}(params: any): any {\n  return null;\n}",
            "example": "// EXAMPLE USAGE\n// {fn_name}(...);",
        },
        ".py": {
            "interface": "# PUBLIC INTERFACE\nfrom typing import Any\n\ndef {fn_name}(params: Any) -> Any:\n    \"\"\"Interface declaration.\"\"\"\n    pass",
            "contract": "# BEHAVIOUR CONTRACT\n# - Pure function",
            "impl": "# CORE IMPLEMENTATION\ndef {fn_name}(params: Any) -> Any:\n    return None",
            "example": "# EXAMPLE USAGE\n# {fn_name}(...)",
        },
        ".go": {
            "interface": "// PUBLIC INTERFACE\nfunc {fn_name}(params any) any",
            "contract": "// BEHAVIOUR CONTRACT\n// - Pure function",
            "impl": "// CORE IMPLEMENTATION\nfunc {fn_name}(params any) any {\n\treturn nil\n}",
            "example": "// EXAMPLE USAGE\n// {fn_name}(...)",
        },
    }
    stub = stubs.get(ext, stubs[".ts"])
    fn_name = "".join(word if index == 0 else word.capitalize() for index, word in enumerate(re.split(r"[-_]", verb_noun)))

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

    header_start, prefix, header_end = COMMENT_STYLES.get(path.suffix, ("/*", " *", " */"))
    header_match = re.search(rf"{re.escape(header_start)}(.*?){re.escape(header_end)}", text, re.DOTALL)
    token_backend = "heuristic"
    if header_match:
        header_text = header_match.group(0)
        impl_text = text[header_match.end():]
        header_tokens, token_backend = count_tokens(
            header_text,
            args.tokenizer,
            args.model,
            args.tokenizer_resource,
        )
        impl_tokens, _ = count_tokens(
            impl_text,
            args.tokenizer,
            args.model,
            args.tokenizer_resource,
        )
        total_tokens = header_tokens + impl_tokens
        ratio = round(header_tokens / total_tokens, 3) if total_tokens > 0 else 0.0
        if args.write:
            metric_lines = [
                f"{prefix} - header-tokens: {header_tokens}",
                f"{prefix} - implementation-tokens: {impl_tokens}",
                f"{prefix} - retrieval-ratio: {ratio}",
                f"{prefix} - tokenizer-backend: {token_backend}",
            ]
            text = replace_metadata_block(text, path, "token-metrics", "\n".join(metric_lines))
            meta["token-metrics"] = (
                f"header-tokens: {header_tokens}\n"
                f"implementation-tokens: {impl_tokens}\n"
                f"retrieval-ratio: {ratio}\n"
                f"tokenizer-backend: {token_backend}"
            )

    score, grade = compute_score(meta)
    print(f"capability-score: {score} (grade {grade})")
    print(f"token-backend: {token_backend}")

    if args.write:
        text = re.sub(r"(@capability-score:\s*).*", rf"\g<1>{score} (grade {grade})", text)
        path.write_text(text, encoding="utf-8")
        print(f"  updated metadata in {path.name}")


def cmd_validate(args):
    path = Path(args.file)
    if not path.exists():
        print(f"ERROR: file not found: {path}")
        sys.exit(1)
    meta = parse_metadata(path)
    registry = resolve_registry(path, args.scope, args.registry)
    failures = validate_module(path, meta, registry if registry.exists() else None)
    if failures:
        print(f"FAIL  {path.name}  ({len(failures)} issue(s))")
        for failure in failures:
            print(f"  - {failure}")
        sys.exit(1)
    score, grade = compute_score(meta)
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

    registry = resolve_registry(path, args.scope, args.registry)
    try:
        ensure_registry_exists(registry, args.scope)
    except FileNotFoundError as exc:
        print(f"ERROR: {exc}")
        sys.exit(1)

    if not args.skip_validate:
        failures = validate_module(path, meta, registry)
        if failures:
            print(f"FAIL  validation failed ({len(failures)} issue(s))")
            for failure in failures:
                print(f"  - {failure}")
            sys.exit(1)

    module_id = meta["module-id"]
    existing_ids = read_registry_ids(registry)
    if module_id in existing_ids:
        print(f"ERROR: collision - {module_id!r} exists")
        sys.exit(1)

    registry_root = registry.parent.resolve()
    file_path = str(path.resolve()) if registry.name == SHARED_REGISTRY_FILE else str(path.resolve().relative_to(registry_root))
    append_registry_row(registry, meta, file_path)
    print(f"registered  {module_id}")


def cmd_status(args):
    registry = resolve_registry(Path.cwd(), args.scope, args.registry)
    if not registry.exists():
        print("no registry found")
        return

    text = registry.read_text(encoding="utf-8")
    print(f"LOCS Registry - {registry}")
    in_table = False
    for line in text.splitlines():
        if line.strip() == "## Registry Table":
            in_table = True
            continue
        if in_table and not line.strip():
            continue
        if in_table and line.startswith("|"):
            print(line)
        elif in_table:
            break


def cmd_bootstrap(args):
    registry = resolve_registry(Path.cwd(), args.scope, args.registry)
    if not registry.exists():
        print("ERROR: registry not found")
        sys.exit(1)

    module_map, graph = extract_registry_data(registry)
    modules = list(module_map.values())
    if args.category:
        modules = [module for module in modules if module.get("category") == args.category]
    if args.domain:
        modules = [module for module in modules if module.get("domain") == args.domain]
    if args.primary:
        modules = [module for module in modules if module.get("primary-capability") == args.primary]

    def sort_key(module: dict[str, str]) -> tuple[Any, ...]:
        score, _ = parse_capability_score({"capability-score": module.get("capability-score", "")})
        return (-(score or 0.0), compute_dependency_depth(module["module-id"], graph), module["module-id"])

    modules = sorted(modules, key=sort_key)[:args.limit]

    print("--- LOCS BOOTSTRAP (Condensed Signatures) ---")
    for module in modules:
        print(
            f"@module: {module['module-id']} | {module.get('primary-capability', '')} | "
            f"{module.get('signature', '')} | score={module.get('capability-score', 'n/a')}"
        )
    print("---------------------------------------------")


def main():
    parser = argparse.ArgumentParser(prog="locs", description="LOCS CLI v1.4")
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
    p_score.add_argument("--tokenizer", choices=["auto", "heuristic", "tiktoken", "transformers", "sentencepiece"], default="auto")
    p_score.add_argument("--model", default="")
    p_score.add_argument("--tokenizer-resource", default="")

    p_val = sub.add_parser("validate", help="Validate module")
    p_val.add_argument("file")
    p_val.add_argument("--registry")
    p_val.add_argument("--scope", choices=["auto", "local", "shared"], default="auto")

    p_reg = sub.add_parser("register", help="Register module")
    p_reg.add_argument("file")
    p_reg.add_argument("--skip-validate", action="store_true")
    p_reg.add_argument("--registry")
    p_reg.add_argument("--scope", choices=["auto", "local", "shared"], default="auto")

    p_status = sub.add_parser("status", help="Registry status")
    p_status.add_argument("--registry")
    p_status.add_argument("--scope", choices=["auto", "local", "shared"], default="auto")

    p_boot = sub.add_parser("bootstrap", help="Context-aware bootstrap")
    p_boot.add_argument("--category")
    p_boot.add_argument("--domain")
    p_boot.add_argument("--primary")
    p_boot.add_argument("--limit", type=int, default=5)
    p_boot.add_argument("--registry")
    p_boot.add_argument("--scope", choices=["auto", "local", "shared"], default="auto")

    args = parser.parse_args()
    {
        "new": cmd_new,
        "score": cmd_score,
        "validate": cmd_validate,
        "register": cmd_register,
        "status": cmd_status,
        "bootstrap": cmd_bootstrap,
    }[args.command](args)


if __name__ == "__main__":
    main()
