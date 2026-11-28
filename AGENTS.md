---
created_on: 2026-10-14 05:37
last_modified: 2026-11-28 13:40
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
- Keep human tree labels intact and align columns by Unicode terminal cells. Break text only at spaces, keep leading indentation, keep tree rails across continuation lines, and keep a word wider than the terminal whole on its own line. Move a description under its label when its column is narrower than 20 cells. Never truncate help text or leave trailing whitespace.
- Keep agent output deterministic and complete. It never reads `COLUMNS` or the terminal size, and no option shortens it. Every line is a key, a list item, or a continuation indented beneath one.
- Report native inherited flags, possible values, explicit defaults, argument requirements, and environment bindings. Do not infer custom parser types from unstable implementation details.
- Binaries never embed the skill. It lives in the CLI's GitHub repository at `skills/<cli>/SKILL.md`, where `<cli>` is the root command's name and the skill's frontmatter `name`. `SkillSource::new` rejects a repository that is not exactly two non-empty `owner/repo` segments without whitespace, and an empty or whitespace-containing branch; `None` adds no command and no alert.
- Bare `<cli> skill` prints these bytes offline, identical in both modes and ending in exactly one newline. `write_skill` propagates I/O failures. A configured skill name or alias collision is an error. Keep the alert wording ``ALERT: Agents must read `AGENT=1 <cli> skill` before using this tool.``

  ```text
  Install the <cli> skill using the Skills CLI:

    npx skills add <owner>/<repo> --skill <cli>

  Skill:

    https://github.com/<owner>/<repo>/tree/<branch>/skills/<cli>
  ```

- `<cli> skill install <dir>` uses Git at invocation time to install the configured branch's skill directory and companion files under `<dir>/<cli>`. Create missing parents, reject an existing destination, and print `installed: <path>` on success in both modes. Include the installer route and its argument in downstream skill coverage.
- `skill_omissions` checks the `SKILL.md` a CLI's tests read from its repository.
- Register `completion <shell>` by default unless `HelpOptions::disable_completion` is set. Reuse a manually registered built-in completion command and reject consumer-defined name or alias collisions when automatic registration is enabled. Use upstream shell names and generators. Generate from a private clone of the full command tree and return output errors.
- Code changes affecting runtime results require corresponding behavioral test changes and at least 90% line coverage; `scripts/` is excluded. Retain tests for real parser help routes and write failures.
- Never publish releases, tags, packages, or production deployments without explicit user authorization.

## Source provenance

The reference implementation is `alexgorbatchev/cobra-help-tree` at revision `2bc93c950a10681dcc916702f61d3b7b52ad602d`. Preserve its MIT attribution. The Rust API uses native clap definitions; Go-specific command mutation and runtime printing hooks are not portable contracts.
