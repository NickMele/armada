# Ported check-engine cases

Behaviour cases harvested from the source repo's check engine and written as **data**, not as
translated test functions. `docs/harvest.md` is the behaviour spec these came from; this directory
is the machine-readable half of it.

**Why data.** The original suite is `run_fn`-injected and charkit has three seams behind a `Ctx`
(`ARCHITECTURE.md` §1.1). Porting the functions would port the harness, which does not fit. Porting
the cases keeps the assertions — the valuable part — and lets the Rust suite drive them through
whatever harness it wants.

**Contamination.** This directory is **not** `tests/fixtures/` and is therefore covered by the
contamination grep (`ARCHITECTURE.md` §2.4). Every path, component and service name here is
neutral. Components are `api`, `ui` and `tools`; applications are `app-a`…`app-d`; the shared
package is `pkg-ui`. Where a case's meaning depends on the *shape* of a path rather than its name,
the shape is preserved and the name is not.

## Schema

One file per subsystem. Every file has the same top level:

```yaml
subsystem: scope
source: check.py
cases: []
excluded: []
```

### A case

```yaml
- id: scope.classify.other-component-only
  trap: SC-5
  note: one line on what the case pins
  given:
    files: ["services/ui/page.tsx"]
    all: false
  expect:
    components: ["ui"]
```

| Key | Meaning |
|---|---|
| `id` | Stable, dotted, unique across the directory. Cite it in a Rust test name. |
| `trap` | Optional. The `docs/harvest.md` trap this case pins — a §5 trap id (`SC-5`, `ST-20`), or a section reference such as `"§6.1"` for the Playwright traps, which are described rather than numbered. Absent means it pins ordinary behaviour. |
| `note` | Optional. One line, only where the case's point is not obvious from its id. |
| `given` | Inputs. Pure-core cases give plain values; cases needing a seam give `run` (below). |
| `expect` | Expected outputs. A case asserts on values, never on argv text — see below. |

### Recorded command output

Cases that exercise `ctx.run` carry a `run` list. Each entry matches an invocation by a
**subsequence of its argv** and supplies the recorded result:

```yaml
given:
  run:
    - match: ["diff", "--name-only"]
      exit: 0
      stdout: "services/api/models.py\n"
    - match: ["status"]
      exit: 0
      stdout: " M services/api/views.py\n"
```

`exit` defaults to `0`, `stdout` and `stderr` to the empty string. An invocation matching no entry
is a test failure, not a silent empty result — a case must account for every command it causes.

### Asserting on invocations

Argv is asserted by **property**, never by transcription, because the original's argv is that
repo's toolchain and charkit's is config. Properties available:

| Property | Meaning |
|---|---|
| `contains` | every listed token appears somewhere in the argv |
| `absent` | none of the listed tokens appears |
| `count` | exactly this many invocations were made |
| `cwd` | the working directory the invocation ran in |
| `env` | environment entries the child must receive, including empty-string values |
| `order` | ids of invocations that must appear in this relative order |

```yaml
expect:
  invocations:
    count: 1
    contains: ["--workers=4"]
    absent: ["--update-snapshots"]
```

## Excluded cases

Every file ends with an `excluded` list. These are assertions from the original suite that are
**vacuous or cannot fail** (`ARCHITECTURE.md` §2.1.1, and `docs/harvest.md` §9). They are recorded
rather than dropped silently, so the omission is visible and so nobody ports them later believing
they add coverage.

```yaml
excluded:
  - id: parse.noise-filter.original
    reason: >-
      With no compiler-error line in the fixture the parser degrades to a fallback whose
      last-line rule already returns the expected answer, so the guard never fires and the
      noise filter is never called. Deleting the filter leaves the assertion green.
    replaced_by: parse.crash.noise-ahead-of-error
```

An entry either names a `replaced_by` case in the same file, or states why no rewrite was possible.

## Invert every one of these once

`ARCHITECTURE.md` §2.1.1: a vacuous assertion is worse than none, because it gets cited as
evidence. Every case here came from a suite that shipped at least four of them. When wiring a case
up, break the behaviour it pins and watch the case fail before trusting it.
