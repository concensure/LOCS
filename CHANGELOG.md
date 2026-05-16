# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.1.1] - 2026-05-16

### Fixed
- Fixed a bug in `locs new` where the `.ts` (TypeScript) implementation stub contained unescaped braces, causing a `KeyError` during string formatting in Python.
- Improved generic language support in `locs new` by ensuring stubs are properly escaped for `str.format()` operations.

## [2.1.0] - 2026-04-10

### Added
- Initial release of LOCS v2.0 framework.
- AST-backed validation for Python and TypeScript.
- Token metrics tracking with multiple backends (tiktoken, transformers, sentencepiece).
- Multi-layered enforcement (Generation, Pre-stage, Pre-commit, Advisory).
- Project-local and shared registry models.
