---
id: 007
title: The scanner should propose, not only report
status: BUILT
module: manifest
raised: real use — scanner run against a real monorepo
---

# 007 — The scanner should propose, not only report

**The user's assessment, and it is the right one.** *"I am quite impressed with what the scanner
was able to find. And now that I see what it was able to find, I do think that it can take it a
little bit further … these are the type of things that are deterministic and should be
identified via script."*

**This does not reverse PLAN.md §5.** *"Do not write a stack-detection engine"* stands. The
distinction that keeps it standing: the scanner may **propose what it can prove**, and must not
guess. Everything below is an exact match on evidence already in the report, and every proposal
is rejectable before anything is written.

Run against a real monorepo, the scan found three workspace candidates, fifteen packages, a
`pnpm-lock.yaml`, sixteen compose services and twenty-two CI steps — and then proposed nothing.

**What is deterministic enough to propose:**

- **The package manager.** A `pnpm-lock.yaml` is proof of pnpm, not evidence of it. Same for
  `uv.lock`, `package-lock.json`, `yarn.lock`, `bun.lockb`.
- **Workspaces.** A directory that resolves its own dependencies is a workspace candidate, and
  the packages nested beneath it belong to it. The scan already computes this.
- **Checks, on an exact name match only.** A workspace whose `package.json` has a script
  literally named `test` may propose it as the test check. `test:changed` and `test:coverage`
  may not — a near-match is a guess wearing a fact's clothes.

**The judgement calls stay the user's**, which is what the interactive pass is for: the scan
proposes, the reader ticks what is right, and what survives is written. In the example, three
workspace candidates were found and only two were real — `scripts` was a toolset, not a
workspace. **No amount of evidence would have settled that**, and that is the line between
proposing and guessing.

**The saving is the point.** A proposal a reader corrects costs no tokens; the same file
authored by an agent costs a session. The agent hand-over stays for the repositories where the
evidence genuinely does not settle it.

---

## What was built

`armada_core::propose` and the tick list in `armada manifest config scan`. The three bullets
above became four proofs, and the exact shape each one takes was decided by what `config verify`
will accept — a proposal that writes a document layer 3 rejects is worse than no proposal at all.

| Proposal | The proof |
|---|---|
| `workspaces: [dir]` | `dir/armada.yml` is there |
| `components.<name>` | `dir` carries a package manifest, and something in it is a check |
| `setup: <pm> install` | the lockfile in that directory names `<pm>` |
| `checks.<name>` | a script or `Makefile` target named **exactly** `<name>` |

**One bullet came out differently from how it was written here, and the difference is the
point.** *"A directory that resolves its own dependencies is a workspace candidate"* — true, and
a candidate is not a workspace. `workspaces:` means *a separate product with a config of its
own* ([`PLAN.md`](../PLAN.md) §4.6), and verify requires every path listed there to contain an
`armada.yml`. Proposing a candidate would write a document that cannot verify, so the proof is
the file rather than the dependency resolution, and every other candidate is proposed as a
component instead. `packages[].name` and `nested` were added to the evidence to support it.

**Three names are deliberately not check names**, each for its own reason: `fmt` / `format`,
because a formatter's bare name rewrites the tree and `armada manifest check` must never mutate
a working copy; `e2e`, because it always arrives with a `cost:` and often an `exclusive:`, and
proposing it without either is proposing a scheduling decision; and `check`, because a
repository's `check` script usually means *run everything*, which would nest a suite inside a
suite.

**Against the `polyglot-monorepo` fixture — which is the raising repository's shape — it
proposes one component and three checks, and says nothing at all about `backend/` or
`scripts/`.** Neither has a script or a target Armada can phrase a command from, so neither
gets a component. The reader never has to untick `scripts`, which is a better outcome than the
one this item asked for.

#### Drift — the scanner run against a repository that already has one · **NOT BUILT**

**Repositories change**: services appear, scripts are renamed, CI is rewritten. Today `scan` is
the verb for a repository with no `armada.yml` and says so. It should also answer the second
question: **is what this file claims still true, and is there anything real it does not
mention?** Both halves matter — a check pointing at a deleted script and a new service nobody
declared are different failures with the same cause.

**Deferred deliberately, and half of the machinery is now here.** `data.proposals` is computed
for a configured repository as well as an unconfigured one, so drift is the comparison of that
list against the resolved config rather than a second scanner. What is not built is the
comparison, the report and the verb surface for it. Until it is:

- `config scan` computes proposals in a configured repository but never offers to write them.
- `config::write` refuses over an existing `armada.yml` rather than merging into it, because a
  merge without a drift report is a rewrite nobody reviewed.
