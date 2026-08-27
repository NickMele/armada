# @armada/desktop

Bridge — the Electron desktop application. Three processes: `src/main` holds
the connections to Fleet, `src/preload` is the whole surface the renderer can
reach, and `src/renderer` draws. `docs/practices/bridge.md` is why it is split
that way and what the split refuses.

## The commands

Each needs `pnpm install` to have run, and nothing else. **None of them needs a
running Fleet**, and none of them starts one — Bridge and Fleet have
independent lifetimes, so a window that opens against no daemon is a state the
app draws rather than an error.

| Command | What it does |
|---|---|
| `pnpm --filter @armada/desktop codegen` | Rewrites `src/shared/generated/` from the registries. Run it after changing `crates/core-model/domain/` or `protocol-version.toml` |
| `pnpm --filter @armada/desktop typecheck` | `tsc -b --force` across all three processes. Silence is success |
| `pnpm --filter @armada/desktop build` | Bundles the three processes into `out/`. Does not package an installer; there is no packager in this workspace yet |
| `pnpm --filter @armada/desktop dev` | Opens the window with reload. The only one that opens a window |

## What `codegen` emits, and what its output means

It reads `crates/core-model/domain/enum-verbs.toml`, `job-statuses.toml`,
`job-fields.toml` and `protocol-version.toml`, and writes two checked-in
TypeScript modules: the verb, glyph and status token each enum variant renders
as, and the two numbers of the protocol version both sides read. **It does not
emit the DTO types** — nothing generates those from `crates/ipc` yet, so a shape change is
still hand-mirrored in `src/shared/protocol.ts`.

The output is checked in on purpose. A generated file that is `.gitignore`d
looks fine locally and is wrong on every machine that did not just run codegen.

It prints one line per file plus a `gap:` line for anything the registries
could not answer. **A gap is not a failure.** It names a variant with no verb,
no glyph or no status token, or a whole vocabulary with no rows at all —
`check_outcome` is one today — and Bridge renders the wire spelling for those
rather than copy invented in a component. The command fails, loudly, only when
a registry file stops being readable or when two registries disagree about
which statuses exist.

## Where the rules are

- `docs/practices/bridge.md` — the three-process split, the security posture,
  and the v1 failures this app exists to escape.
- `docs/practices/protocol.md` — the seam to Rust: version skew, the DTOs, and
  the second socket that carries one Job's turns.
- `docs/contracts/design-system.md` — nothing invented. A surface builds from
  `@armada/tokens` and `@armada/components` alone.
