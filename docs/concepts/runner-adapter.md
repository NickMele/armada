# Runner adapter

**What it is:** a declarative description of one test runner — how to detect
it, how to run one test by name or by another shape, and how to read its
output — that lets `draft_fix` reach a runner nobody has hand-configured for
this repository.

Distinct from [Adapters](../contracts/adapters.md): that contract governs code
Armada calls through a Rust trait, chosen by who decides when it runs. A
runner adapter is data with no code behind it — a file signature, a template
string and a regex — read by `checks_runner`, which already exists.

## Why `draft_fix` needs one

A Drone that hits a test already failing on main names the Check and the
test; Fleet runs just that test against a checkout of main before drafting
anything, using the command the Check's `one_test` declares
(`crates/fleet/src/fixing.rs`, #999). Only a Check that declares `one_test`
can be reached this way. In this repository that is `test`, `acceptance`,
`components_test`, `screens_test` and `desktop_test` — every other Check has
no way to isolate one test, so a Drone hitting a pre-existing failure there
has no way to tell its own change apart from a break already on main.

A hand-written `one_test` line does not generalize past the runner it was
written for, and a small enum of known runners in Fleet's own code does not
generalize past whichever ones somebody added. A runner adapter is data
instead: something Fleet can match against a repository without anybody
having configured that repository for it first.

## The schema

### `detect`

| field | type | required | default | meaning |
|---|---|---|---|---|
| `manifest` | file path | no | none | manifest file the dependency is read from |
| `dependency` | string | no | none | package name to find inside that manifest |
| `files` | list of globs | no | none | files whose presence marks this runner |

### `requires`

| field | type | required | default | meaning |
| --- | --- | --- | --- | --- |
| `library` | string | yes | none | the runner's package name |
| `version` | semver range | yes | none | version range this adapter was authored against |

### `commands`

| field | type | required | default | meaning |
|---|---|---|---|---|
| `run` | template string | no | none | run the whole suite |
| `one_test` | template string, `{test}` | no | nearest supported shape | run exactly one test by name |
| `run_failed` | template string, `{failed}` | no | nearest supported shape | rerun a named failed set |
| `run_changed` | template string, `{files}` | no | nearest supported shape | run tests touching changed files |
| `run_group` | template string, `{group}` | no | nearest supported shape | run one named group or project |
| `run_pattern` | template string, `{glob}` | no | nearest supported shape | run tests matching a name glob |

### `output`

| field | type | required | default | meaning |
|---|---|---|---|---|
| `line` | regex, named groups `status`, `test` | yes | none | parses one output line into a structured result |

```yaml
# illustrative only — naming and versioning are not decided, see Deferred below
detect:
  manifest: "package.json"
  dependency: "vitest"
  files: ["vitest.config.*"]

requires:
  - vitest ^1.0.0

commands:
  run:         "pnpm -C {dir} test"
  one_test:    "pnpm -C {dir} test -- -t {test}"
  run_failed:  "pnpm -C {dir} test -- --retry {failed}"
  run_changed: "pnpm -C {dir} test -- --changed {files}"
  run_group:   "pnpm -C {dir} test -- --project {group}"
  run_pattern: "pnpm -C {dir} test -- -t {glob}"

output:
  line: '^\s*(?<status>✓|×)\s+(?<test>.+?)\s+\d+ms$'
```

### Each shape is one command, never `run` plus a suffix

Every field in `commands` — `run`, `one_test`, `run_failed`, `run_changed`,
`run_group`, `run_pattern` — is its own complete, independently written
string, the way the example above shows. None is computed by appending
something to `run`.

**Decided, not left open.** An append-only suffix was the shorter-looking
alternative — track `run` automatically, write less — and it was tried for
real: `desktop_test`'s `run: pnpm -C apps/desktop test` with a suffix of
`-- -t {test}` appended. It silently ran the *entire* suite instead of one
test, exit code `0`, reading as an ordinary pass (#1205). pnpm's own `--`
forwarding collides with vitest's argument parsing in a way nothing about the
suffix syntax itself would warn a reader of. The working command,
`pnpm --dir apps/desktop exec vitest run -t {test}`, isn't `run` plus
anything — it's a different invocation. A full, independently-verified
command per shape costs more typing and can drift from `run` if `run` changes
later; an appended one can silently run the wrong thing while still looking
right. Between "more to write" and "quietly wrong," this schema always takes
the first.

## What is built

`run_changed`, from shipped descriptions, and enough detection for the Manifest
proposer to name a runner on a repository nobody has configured.

| Piece | State |
|---|---|
| A Check names its runner, and a narrowed run resolves `run_changed` from it | built |
| `detect.command` — the program a workspace's own runnable names | built, and the only detection there is |
| `detect.manifest`, `detect.dependency`, `detect.files` | read by nothing |
| `one_test`, `run_group`, `run_pattern`, `run_failed` | no caller |
| Learning, verification, publishing | not started |

**Detection reads what a script runs, not what is installed.** A Scan records
each runnable's command verbatim, so `"test": "vitest run"` gives the runner
away — and a package holding vitest while its `test` script runs something else
would make a dependency list say the opposite. That is also why the one signal
built is the one Scan already carries; nothing reads a dependency list yet.

**`output` is not read, and `--passWithNoTests=false` is why that is safe for
now.** The third verification state below needs a runner's output parsed; vitest
can be told to report it as an exit code instead, so the shipped description
asks for that and the tri-state holds without a parser. A runner that cannot say
it matched nothing by its exit code needs `output` read before it can be trusted
to narrow.

## The six shapes are fixed

Fleet's caller code knows how to invoke exactly these six. Where an adapter
omits one, Fleet falls back to the nearest shape it has, ordinarily plain
`run`. A runner needing a seventh shape is a schema change and a change to
every call site that reads `commands`, not something an adapter author
reaches by editing a file.

## Data, not code

Every field above is a file signature, a template string, or a regex with
named capture groups. None of it is a script Fleet executes as logic — Fleet
only ever interpolates a test name into a fixed template and reads the result
back through a regex.

That constraint is what makes a learned adapter safe to keep and, later,
share: nothing an adapter's author writes ever runs as a program on Fleet's
side. It holds from this schema's first version, because the six shapes above
and the one regex field are the whole surface an adapter can fill in — adding
a seventh shape or an executable field is the schema change described above,
not a config edit.

## Learning one Fleet has not seen

| step | what happens |
|---|---|
| unmatched | no `detect` block matches the repository's manifest, dependency or files |
| docs lookup | reference material for the detected runner and version is fetched before drafting anything, keyed the way a Context7-style lookup keys on a package and its version — real documentation, not scraped examples alone |
| draft | a bounded model call proposes a candidate adapter from that reference material |
| verify | the candidate's `one_test` line runs against a fixture proven to isolate one named test |
| confirm | a person sees the candidate and the verification result once, before it is trusted |
| trusted | the adapter is kept and reused without repeating the draft-and-verify pipeline |

Confirming once follows the same discipline as onboard-repo's guess-then-confirm
bar and fixture-author's fixtures-before-detectors rule: a guess is attributed
to what produced it, and nothing is trusted before somebody has seen it proven.
The output regex is modeled on GitHub Actions' problem matchers — one pattern,
named groups, no parser code — for the same reason those exist: to read a
tool's output without writing a parser for every tool.

### Verification is tri-state, not a boolean

| outcome | what it means |
|---|---|
| ran and passed | the candidate isolated exactly the named test, and it passed |
| ran and failed | the candidate isolated exactly the named test, and it failed |
| matched nothing | the candidate exited as if it passed, without running the named test at all |

A candidate that only distinguishes pass from fail repeats #1204: on both
runners measured so far, a filter that matches nothing exits the same as a
filter that matched and passed. Verification has to catch the third outcome
against a fixture proven to isolate one test, because a live suite makes a
false pass indistinguishable from a real one.

## Where an adapter comes from

An adapter reaches a repository by one of three routes, and the third feeds the
first.

| Route | When it applies |
|---|---|
| Shipped | Armada carries adapters for the runners it already knows. A repository on one of them is configured by being detected |
| Learned | No shipped adapter's `detect` matches. The draft-and-verify path above produces a candidate, a person confirms it once, and Fleet keeps it |
| Published | A learned adapter is sent back to Armada's own repository, and becomes a shipped one by pull request |

**A learned adapter is worth keeping beyond the machine that learned it.** The
work of drafting one is a model call, a fixture run and a person's attention,
and every repository on that runner afterwards would repeat all three. Sending
it back is what makes the second repository on a runner cost nothing.

**Publishing is a submission to the repository Armada itself is developed in**,
not to a service Armada operates. Nothing has to be hosted, every submission is
public and reviewable the moment it arrives, and an author needs the account
they would need to file an issue. What that costs is the lookup: asking whether
a runner is already known means reading what is in the repository, so it is not
free the way an indexed registry's answer would be — and a draft-and-verify run
that a lookup would have avoided is a model call, a fixture run and a person's
attention.

**The pull request is how a published adapter becomes a shipped one.** Receiving
one opens a change against Armada rather than adding it to a live set, so an
adapter everybody gets is reviewed the way every other shipped default is. That
is also what keeps `detect` honest: an adapter whose detection is too broad
matches repositories it was never proven against, and the review is where that
is caught.

A repository may carry an adapter of its own, and it is read ahead of a shipped
or learned one. That is the answer to a repository running a common runner in an
uncommon way — it says so locally rather than the shipped adapter growing a case
for it.

## Deferred: sharing

The target is settled above. Nothing about naming, versioning or trust tiers is,
and what a publish layer would need is already true of this schema, so building
one does not require reopening it:

- a stable identity and version per adapter — the example above's comment
  is illustrative only, not a decided naming scheme
- `detect` staying pure data, so a registry could index adapters by it without
  executing anything to do so
- nothing in `detect`, `requires`, `commands` or `output` that only makes
  sense scoped to one repository
