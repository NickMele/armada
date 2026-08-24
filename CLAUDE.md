# Armada

A macOS app that dispatches coding agents against git repositories and verifies
their work before advancing them. Rust daemon, Electron app, one repo.

Procedures govern how you work on it — read before starting, not when stuck:
https://app.notion.com/p/3c0173a35eb9800a9da2e6b7f1403ab1

- `cargo xtask verify-foundations` is **red on purpose** until M0 is finished.
- **The acceptance test must fail** for all of M0. A Stop hook enforces it.
- **v1 is deleted.** A bare file path in a doc means `git show v1-final:<path>`.

| Looking for | Go to |
|---|---|
| Working a milestone step, a workflow, the voice, a Notion edit | `.claude/skills/` |
| Rust, Bridge, or the seam between them | `docs/practices/` |
| What was measured, and what was not | `docs/spikes/` |
| Subagents | `.claude/agents/` |

**A CLAUDE.md routes. It does not explain.** Anything longer than a pointer
belongs in a practice doc, a skill, or Notion — each already the authority, so a
copy here only drifts. 50 lines is refused, 30 asks. v1's agent file reached 328
by accretion, one reasonable paragraph at a time.
