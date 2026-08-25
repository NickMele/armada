# Armada

A macOS app that dispatches coding agents against git repositories and verifies
their work before advancing them. Rust daemon, Electron app, one repo.

- `cargo xtask verify-foundations` is **red on purpose** until a milestone is
  finished. Do not chase green — read what each failing line names.
- **The acceptance test must fail** for all of M0. A Stop hook enforces it.
- **v1 is deleted.** A bare file path in a doc means `git show v1-final:<path>`.
- **Never write an address into this repository.** It is public and the design
  workspace is not. Name what a thing is, not where it is.

| Looking for | Go to |
|---|---|
| A binding rule — architecture, errors, config, design, prompts | `docs/contracts/` |
| Rust, Bridge, or the seam between them | `docs/practices/` |
| What is open, and what is waiting on it | `docs/OPEN.md`, generated |
| A value — tokens, icons, settings | `packages/`, `crates/config/settings.toml` |
| What was measured, and what was not | `docs/spikes/` |
| What is being built, and in what order | GitHub issues, grouped by milestone |
| Concepts, journeys, and the decision record | The design workspace — read `.claude/skills/armada-docs` first |
| Skills and subagents | `.claude/skills/`, `.claude/agents/` |

Everything written down is in `docs/INDEX.md`, and the gate refuses a document
that is not.

**A CLAUDE.md routes. It does not explain.** Anything longer than a pointer
belongs in a contract, a practice doc or a skill — each already the authority,
so a copy here only drifts. 50 lines is refused, 30 asks.
