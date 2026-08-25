# @armada/tokens

**Authored here.** `src/*.css` is the authority on every design value in
Armada. Notion is the authority on the decision and the reasoning behind one —
a row whose value disagrees with the CSS is stale, not right.

A value gets decided while looking at a rendering. So the design project draws
first and proposes a change as a diff against `src/`, carrying the reasoning
that goes in the comment. Nobody carries a zip.

| Path | What it is |
|---|---|
| `src/styles.css` | The cascade. The import order is load-bearing and is the only place it is declared |
| `src/*.css` | The tokens, and the argument for each value |
| `src/base.css` | **Not a token file** — it consumes tokens and declares none. Excluded from the generator |
| `tokens.css` | Generated. The cascade, concatenated, comments intact |
| `tokens.json` | Generated. Every token, its source and its note. Read by the primitive spec test |
| `tailwind.theme.js` | Generated. Theme keys named after the tokens, values `var(--token)` |

```
cargo xtask verify-tokens          # fails if a generated file is stale or hand-edited
cargo xtask verify-tokens --write  # the only way to change one
```

## Refetching from the design project

Read the files straight out of the Armada Mockups design project with the
`DesignSync` MCP (`list_files`, then `get_file` per path) and write them here.
The signed download URLs a handover script carries expire about an hour after
they are minted, so a script that worked yesterday deletes the local copy and
then 404s. That happened twice. The MCP has no expiry.

Check every file, not only the one you were told changed — a half-refetched
set is a mixed vintage, and nothing downstream can tell.

Adding a token file means adding an `@import` to `styles.css` **and**
classifying it in `xtask/src/tokens.rs`. An unclassified file, or a token whose
name matches no entry in the theme table, fails the check rather than being
guessed at.

**Do not seed the generator from the Armada Tokens rows in Notion.** That
inverts the direction of authority, and every later regeneration diffs against
the wrong thing.
