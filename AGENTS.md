---
created_on: 2026-10-14 05:37
last_modified: 2026-10-14 05:37
status: current
---

# cli-command-kit

This library renders help and checks usage guides against native clap command definitions. Inherit the shared rules in `../AGENTS.md`.

## Commands

- Full checks: `just check`.
- Tests: `just test`.
- Format: `just format`.
- Coverage: `just coverage`.

## Contracts

- Render private command clones. Never mutate a caller's command tree or read application files while rendering.
- Keep human tree labels intact and align columns by Unicode terminal cells. Preserve grapheme boundaries when clipping descriptions.
- Keep agent output deterministic and untruncated by default. Optional clipping preserves complete field names and the guide-reading alert.
- Report native inherited flags, possible values, explicit defaults, argument requirements, and environment bindings. Do not infer custom parser types from unstable implementation details.
- `write_skill` writes supplied content verbatim and propagates I/O failures. A configured skill name or alias collision is an error.
- Code changes affecting runtime results require corresponding behavioral test changes and at least 90% line coverage; `scripts/` is excluded. Retain tests for real parser help routes and write failures.
- Never publish releases, tags, packages, or production deployments without explicit user authorization.

## Source provenance

The reference implementation is `alexgorbatchev/cobra-help-tree` at revision `2bc93c950a10681dcc916702f61d3b7b52ad602d`. Preserve its MIT attribution. The Rust API uses native clap definitions; Go-specific command mutation and runtime printing hooks are not portable contracts.
