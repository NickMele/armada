# The code graph

**What it is:** How a change here is checked against GitNexus, the code graph of
this repository — what to ask it before an edit and before a commit, how to
build it, and what it cannot see.

---

Read this before changing a function, type or method that something else calls.

## What it is for

**A grep finds the name. The graph finds what depends on it.** Renaming a field
on a DTO reaches a Rust handler, the generated TypeScript type and a screen that
reads it. A text search finds the first and misses the rest.

| When | Ask | Answers |
|---|---|---|
| Before editing a symbol | `impact`, upstream | Its callers, the flows it sits on, and a risk level |
| Before renaming one | `rename` | Every reference, through the call graph rather than the text |
| Before committing | `detect_changes` | Which symbols and flows the diff touched |
| Finding where something happens | `query`, then `context` | The flow, then one symbol's callers and callees |

The skills under `.claude/skills/gitnexus-*` carry the procedure for each.
`gitnexus-guide` is the one to read first.

## Building it

```sh
pnpm gitnexus:index
```

**What it does:** reads every source file and writes the graph to `.gitnexus/`
at the repository root, which is ignored. It takes about 300 MB.

**When you run it:** once after cloning, and again when a tool says the index is
stale. A worktree is its own repository to GitNexus and needs its own index.

**What it needs:** Node and pnpm, and a network the first time — `pnpm dlx`
fetches the pinned version. Nothing else, and no global install.

**What its output means:** it ends by printing the symbol, relationship and flow
counts. A failure naming `@ladybugdb/core` or `tree-sitter` is an install
script pnpm refused to run; the script passes the flags that allow those three.

**It never writes to `AGENTS.md` or `.claude/skills/`.** A bare `gitnexus
analyze` does both — it appends a block to `AGENTS.md` that fails the fifty-line
gate on `CLAUDE.md`, and overwrites the committed skills with whatever version
ran. The script passes `--skip-agents-md --skip-skills` so neither happens.

## The tools in a session

**`.mcp.json` declares the `gitnexus` server, pinned to the same version as the
script.** Claude Code asks once whether to enable it. Without it, every skill
names a CLI fallback, and `pnpm gitnexus <command>` runs any of them.

## What it cannot see

**`risk: UNKNOWN` is not low risk.** An empty caller set also means callers the
index could not resolve: a call through a plain object, dynamic dispatch, or the
seam between Rust and TypeScript, which crosses as JSON. Confirm with a text
search before treating the symbol as unused.

**A stale index answers for the code as it was.** After a rebase or a pull,
rebuild before trusting an empty answer.

## Upgrading it

**The version is pinned in two places: `package.json` and `.mcp.json`.** Move
both in one commit, then run `pnpm gitnexus analyze --skip-agents-md` once
without `--skip-skills` to refresh the committed skills, and read their diff.
