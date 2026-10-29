---
created_on: 2026-10-14 05:37
last_modified: 2026-10-29 12:49
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
- Keep agent output deterministic and untruncated by default. Optional clipping preserves complete field names and the skill alert.
- Report native inherited flags, possible values, explicit defaults, argument requirements, and environment bindings. Do not infer custom parser types from unstable implementation details.
- Binaries never embed the skill. It lives in the CLI's GitHub repository at `skills/<cli>/SKILL.md`, where `<cli>` is the root command's name and the skill's frontmatter `name`. `SkillSource::new` rejects a repository that is not exactly two non-empty `owner/repo` segments without whitespace, and an empty or whitespace-containing branch; `None` adds no command and no alert.
- `write_skill` prints these bytes, identical in both modes and ending in exactly one newline, and propagates I/O failures. A configured skill name or alias collision is an error. Keep the alert wording ``ALERT: Agents must read `AGENT=1 <cli> skill` before using this tool.``

  ```text
  Install the <cli> skill using the Skills CLI:

    npx skills add <owner>/<repo> --skill <cli>

  Skill:

    https://github.com/<owner>/<repo>/tree/<branch>/skills/<cli>
  ```
- `skill_omissions` checks the `SKILL.md` a CLI's tests read from its repository.
- Code changes affecting runtime results require corresponding behavioral test changes and at least 90% line coverage; `scripts/` is excluded. Retain tests for real parser help routes and write failures.
- Never publish releases, tags, packages, or production deployments without explicit user authorization.

## Source provenance

The reference implementation is `alexgorbatchev/cobra-help-tree` at revision `2bc93c950a10681dcc916702f61d3b7b52ad602d`. Preserve its MIT attribution. The Rust API uses native clap definitions; Go-specific command mutation and runtime printing hooks are not portable contracts.
