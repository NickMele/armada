# Armada

A macOS app that dispatches coding agents against git repositories and verifies
their work before advancing them. Rust daemon, Electron app, one repo.

- `cargo xtask verify-foundations` is **red on purpose** until a milestone is
  finished. Do not chase green — read what each failing line names.
- **The acceptance test is a milestone's own claim**, written before the code.
  `docs/practices/acceptance-tests.md`.
- **v1 is deleted.** A bare file path in a doc means `git show v1-final:<path>`.
- **Never write an address into this repository.** It is public and the design
  workspace is not. Name what a thing is, not where it is.

| Looking for | Go to |
|---|---|
| **What Armada is for, and what it is not** | **`docs/scope.md`** |
| A binding rule — architecture, errors, config, design, prompts | `docs/contracts/` |
| Rust, Bridge, or the seam between them | `docs/practices/` |
| What is open, and what is waiting on it | `docs/OPEN.md`, generated |
| A value — tokens, icons, settings | `packages/`, `crates/config/settings.toml` |
| What was measured, and what was not | `docs/spikes/` |
| What is being built, and in what order | GitHub issues, grouped by milestone |
| What a thing is, and how it is used | `docs/concepts/`, `docs/journeys/` |
| Skills and subagents | `.claude/skills/`, `.claude/agents/` |

Everything written down is in `docs/INDEX.md`, and the gate refuses one that is
not. `ARCHITECTURE.md` is the map. `CLAUDE.md` symlinks here — a copy drifts.

**This file routes. It does not explain.** Anything longer than a pointer
belongs in a contract, a practice doc or a skill. 50 lines is refused, 30 asks.
