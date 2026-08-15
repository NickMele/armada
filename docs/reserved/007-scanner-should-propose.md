---
id: 007
title: The scanner should propose, not only report
status: RESERVED
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

#### Drift — the scanner run against a repository that already has one

**Repositories change**: services appear, scripts are renamed, CI is rewritten. Today `scan` is
the verb for a repository with no `armada.yml` and says so. It should also answer the second
question: **is what this file claims still true, and is there anything real it does not
mention?** Both halves matter — a check pointing at a deleted script and a new service nobody
declared are different failures with the same cause.
