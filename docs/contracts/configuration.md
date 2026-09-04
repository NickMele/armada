# Configuration

**Kind:** contract. **Governs:** how a setting is classified, how two layers of
configuration combine, and what a Job records about the configuration it ran
under. Read before writing anything in `crates/config`.

The settings themselves are `crates/config/settings.toml` — the crate that
reads them owns the list. Nothing here restates it, because two copies of a
settings list is how the second one goes stale.

Its open questions are in that file's own `## Open questions` section and in
`docs/OPEN.md`, not here.

---

**Adapters are a separate spec.** It covers `AgentHarness`, `VCS`, `Secrets`,
`ModelClient` and `Docker`, and answers where adapter configuration lives.

**Anything derivable from code is generated, never hand-kept.** The crate graph
is the worked example — if it is ever mechanised it comes from `cargo metadata`,
not a typed-out copy — but the rule is general, and it is why this document
carries no settings table. Two sources is how the second one goes stale, and a hand-maintained copy of something the compiler already knows is a second source with no author.

## The three rules

**Config tiering rule.** Revised 22 Aug 2026 with the Guild split. A setting belongs to **Machine** or to **Kit**, and the test is one question: *does a project-level version of this setting make sense?* A project adds a Skill — ordinary, so Skills are Kit. There is no project-level version of how loudly Armada notifies you, so routing is Machine. **Only a Kit setting has a Manifest tier.** A Machine setting has one value, no merge and no second tier.

**Kit sets defaults, not ceilings.** Kit holds a default; a Manifest declares its own value; they merge. A Manifest may **extend or restrict** — nothing above the Manifest constrains anything. The **config direction rule** that used to sit here, which held that config narrows inside a rule and never inverts one, was **withdrawn 22 Aug 2026**. It was invented to police a merge, so it only ever made sense for Kit, and it was applied to everything.

**Merge strategies are the whole mechanism.** A merge strategy governs how two lists or two scalars combine. With the direction half withdrawn there is no second, legality-shaped question layered on top of it: a merge that combines correctly is applied. What survives of polarity is a *description* — see the section below.

**Adapter rule.** An adapter is code Armada itself calls deterministically. Skills, MCP servers and Plugins are tools the Drone chooses to call during its own reasoning — a separate axis, which stays in Kit and Manifest as originally scoped. Secrets is Fleet-side only; a Drone never holds a callable secrets tool. The full statement is in the Adapters spec.

## Polarity is a description, not a legality rule

Revised 22 Aug 2026. The field formerly called **Narrowing rule** was invented to make the withdrawn direction rule structural — it said *which direction is legal*, and a merge moving the wrong way was rejected at config load. With the rule withdrawn, no direction is illegal on the Kit → Manifest axis.

The field survives, renamed `peer_polarity`, because a different question still needs answering: when several **peer Manifests** gate one Job, most-restrictive-wins has to know which direction is stricter. That is Manifest → Manifest and is untouched by the withdrawal.

The worked example still earns its place, because two settings can look identical in every field and merge in opposite directions:

| Setting | Merge strategy | What appending does |
| --- | --- | --- |
| **Allowlist (safe ops)** | `APPEND_AND_REMOVE` | **widens** — a Manifest grants permission the Kit did not |
| **Destructive-op list** | `APPEND` | **narrows** — a Manifest requires confirmation the Kit did not |

So each setting carries a **peer polarity** alongside the merge strategy: `Lower is stricter`, `Higher is stricter`, `Intersect (allow-shaped)`, `Union (deny-shaped)`, `Member of the Kit set`, `Unconstrained — cosmetic`, or `n/a — single layer`. Merge strategy says *how values combine*; peer polarity says *which way is stricter*, which is what most-restrictive-wins consults when several Manifests gate one Job. Neither rejects anything at config load.

**In code the polarity newtypes survive, scoped to peer resolution.** `Cap<T>` resolves by `min` across peers, `Floor<T>` by `max`, an allow-shaped list by intersection, a deny-shaped list by union. What they no longer do is make a Kit → Manifest direction unconstructible.

**What this leaves unwatched**, filed as findings rather than replaced: nothing prevents a Manifest removing a Judge trigger, selecting a model outside the Kit set, removing a required destructive-op confirmation, or defining a Command that bypasses the allowlist and with it the denial record.

## Three more rules, decided after the review

**Each owner declares the config it needs; nothing is handed a central schema.** This was raised as a possible inconsistency — Doctor owns no checks and its modules self-report health, while configuration looked centralised — and it is not an asymmetry. The unknown-key decision below is what settles it: once a config file is a map of owner sections, each strict inside its own parser and opaque to the rest, the schema *is* decentralised, and config and health follow the same rule — **the unit declares, the aggregator renders.** Kit and Machine are held to the same shape as Manifest.

A central schema would also have broken the adapter promise outright: a new Secrets provider's vault reference and endpoint would have had to be added to core, so adding an adapter *would* touch core and the trait would have bought nothing. See **Adapters**.

**A Job persists a flat resolved record, not a typed snapshot.** Fleet writes one map of `key → (value, source)` per Job at spawn — source being `kit`, `machine`, `manifest` or `default` — stamped with the schema version. **The stamp is for interpretation and is never used to deserialize.** A flat record has no schema to break, so there is no migration path and no version handshake.

`CONFIG_VERSION`, proposed as an analogue of `PROTOCOL_VERSION`, was rejected as the wrong shape. `PROTOCOL_VERSION` is a handshake: two live parties must agree or refuse to talk. A config snapshot has no second party. At a gated checkpoint Fleet re-reads the live half against the current schema and uses the frozen half for display only, so an old blob is never type-deserialized and the failure the version tag was invented to prevent cannot occur.

This is also what makes *"why did this Job behave that way"* answerable after the fact — the question v1 could not answer — and it is the resolved-config-plus-source-tag surface the merge-strategy decision already promised at Job-detail level.

**An unknown key in `armada.yml` hard-fails, and the file is namespaced by owner.** Each owner gets a top-level section, strict inside its own parser and opaque to every other, so a Fleet-owned key in a Manifest file is *somebody else's section* rather than an unknown one.

The decisive argument is tier moves. If unknown keys were ignored, a key written today for a `Machine` setting would do nothing — and the day that setting moves to `Kit → Manifest`, it would wake up and change behaviour on a repo nobody had touched. Under hard fail that key could never have been written, so the tier move is safe by construction.

**The example this argument was worked through no longer describes anything, and the argument survives without it.** It was budget moving from `Machine` to `Kit → Manifest`. Budget is Machine and stays there — a Manifest has no budget cap, which is what the Kit/Machine split is for — so that particular move cannot happen. Any Machine row that could move serves as well; what the argument rests on is that a tier move is possible at all, not on which row makes it.

The cost is the collision this project has already paid once: a file written by one module and parsed by another with `deny_unknown_fields`. That is precisely why the answer is namespacing per owner rather than handing the file a single parser.

## One glob dialect, and it is stated here

Added 29 Aug 2026, with `checks.<name>.when`. A path pattern anywhere in Armada's configuration is read by `core_model::PathPattern` and by nothing else, and this is the whole of the dialect:

| Written | Matches |
| --- | --- |
| literal text | itself, exactly |
| `*` | any run of characters inside one path segment, **never** a `/` |
| `**` | zero or more whole path segments |

`/` separates segments. Patterns are repository-relative — a leading `/` is refused, not stripped. A path is matched **whole**: a bare directory name matches only a file of exactly that name, and no prefix is inferred from it. `**` is legal only as an entire segment, so `packages/**` and `**/*.rs` parse and `packages/**x` does not.

**Every other metacharacter is refused by name at load.** `?`, `[`, `{` and a leading `!` are syntax in the dialects an author is likely to be thinking of, and reading them as literal text gives a pattern that matches nothing and says nothing about why. The refusal names the character.

The rule this exists for: **`packages/**` and `packages/*` differing silently is a Check that stops running and nobody notices.** One dialect, stated once, is what makes that difference readable rather than discovered. A second dialect anywhere in configuration is a change to this section first.

**An absent pattern list is never an empty one.** Where a key is optional — `checks.<name>.when` is the first — absent means *always* and the empty list is refused, because one value with two opposite readings is the failure this whole section is about. In code that is `Option<Covers>` and a `Covers` that cannot be built empty.

## The first `armada.yml` section that is a dial rather than a registry

Added 3 September 2026 with `drone.quiet_after_seconds` and `drone.poke_limit`,
which are the middle tier of a Drone's patience. Two things this section settles
because the file had never carried a value of this kind.

**A section is named for what it configures, not for who enforces it.** The
namespacing rule above says *per owner*, and the reason it gives is collision —
a key one module writes and another parses. Naming by topic answers that reason
as well, and it is what every section the Manifest concept page already
describes does: `permissions`, `secrets`, `ports`, `skills`. `settings.toml`
files both rows under Fleet as owner, and a `fleet:` section here would be a
section that could never hold anything else, because every Fleet-owned row with
a Manifest tier at all is about a Drone's conduct and Fleet's own dials are
`Machine`-scoped. So it is `drone:`.

**A tier writes a value under the same name every other tier writes it under.**
A step says `quiet_after_seconds` and so does the repository. One value spelled
two ways is a vocabulary split that becomes visible only when somebody moves a
number from one file to the other and it stops working.

| Where | Read by | When |
| --- | --- | --- |
| `crates/armada/src/serve.rs` | the composition root | daemon start |
| `armada.yml`'s `drone:` | `config::Manifest` | daemon start and on every save, resolved at every step boundary |
| a WorkflowDef step | `config::Step` | frozen at Job creation, resolved at every step boundary |

`fleet::Liveness::at` is the only place that order is written. **Each half falls
back on its own** — a repository stating a threshold and no poke budget inherits
the budget, and a step may override either without restating the other.

**`Live` reaches the file, and stops at the step boundary.** This paragraph said
the opposite until `#430`: `armada.yml` was read once at daemon start, so the
resolution was live against a file that was not, and editing it changed nothing
until a restart. `crates/armada/src/watching.rs` now watches the file and
`crates/config/src/live.rs` adopts what a re-read may move, so a Job whose step
declares nothing follows an edit into its next step.

**Only the keys this table marks `Live` move, and only at a boundary.** A
`checks:` or `commands:` edit is *Frozen for the Job* — every workflow was
resolved against the registry the file declared at start, and adopting a new one
would leave a resolved step pointing at a Check the Manifest no longer agrees
with. Such a change is named as needing a restart rather than swallowed — on the
daemon's console, and, since `#446`, on the wire as well, so a person running
Bridge learns it too. And the live pair is resolved once per step and held for as
long as the Drone is, so a save cannot move the terms a running step began under.

**A save that will not parse changes nothing.** The last good configuration stays
in force and the refusal is reported. A fleet that stopped because somebody
mistyped a number would be worse than one that ignored the edit.

**The refusal is a standing reading, not a line that scrolls past.** Until `#446`
it was printed and nothing else, which meant a person running Bridge edited the
file, saw no change and had nowhere to find out why. `get_manifest_reading`
answers for Fleet's last read and `manifest.reread` pushes it, so a Bridge opened
a minute after the save still learns that the file was refused — and every fault
crosses, not the first, because a person correcting a file from a message naming
one fault meets the next one on the following save.

## Two tiers of path boundary, and only one of them is configuration

Added 3 September 2026, with `#417`. A step's `exclude_paths` held two kinds of entry and refused both the same way, which is why a Drone that found the right fix in a fenced-off file had nowhere to go: the declaration was refused mechanically, and the widening it would have been pointed at grows the Job's `write_targets` rather than the list that refused it.

| | Ordinary | Absolute |
| --- | --- | --- |
| Where it is written | `evidence_scope.exclude_paths`, per step, in a workflow definition | Nowhere. Compiled into `verification` |
| What it is | A boundary somebody drew before anybody had read the code | Secrets, what decides which checks run, what decides how the work is judged |
| At `declare_scope` | Refused, and the refusal names `request_scope` | Refused, and the refusal says asking will not change it |
| At `request_scope` | The Judge is asked whether the paths belong to the step | Refused before any call. There is no answer that lifts it |
| At the gate | Answered over the declaration, and a cleared path passes | Answered over the declaration **and the footprint** |
| Which steps it reaches | The step that declared one, and no other | **Every step**, declared or not — the gate reads the worktree on all of them |

**The absolute tier is not a key, and that is the whole of why it holds.** Every path in it lives inside the repository a Drone has a worktree of — `.git`, `.armada`, `armada.yml`, `.env` and its family. A list naming them from inside that same repository could be edited by the thing it denies, and a Judge that could lift the rules it is judged by is not a boundary. There is no file to edit and no key to widen.

**`.armada/artifacts/` is inside `.armada` and is not a boundary.** It is where seven shipped workflows send a step's deliverable, by a `mechanical_checks[].target` a Drone did not choose and cannot move, and where Fleet opens the file it puts in the Judge's brief. A boundary that refused it would refuse the work rather than protect anything — the same test `Cargo.toml` is kept out by. It is compiled in beside the boundary it narrows, so it is not an exception a caller can supply and there is still no key to widen.

**A forge's continuous-integration directory is not in it, and cannot be as things stand.** Naming one inside `verification` is `no_vendor_literal_outside_adapters` exactly — a forge is the adapter layer's to know, and that rule has no exception mechanism by design. Reaching it would mean the VCS adapter declaring where its forge keeps that configuration and Fleet handing it down, which is a change to `adapter-traits` and `adapters`. Until then a step's own `exclude_paths` is where a repository names it, in the tier a Judge may lift.

**A repository cannot add to it.** The obvious candidate in this repository is `xtask/src/rules.rs` — the gate's own rules — and it is not covered. Adding a per-workflow absolute key is a schema change and is not built.

**Build configuration is deliberately ordinary.** `Cargo.toml`, `package.json` and a Makefile are what `check_config_edited` flags, and none of them is absolute: they are files ordinary work edits constantly, and a boundary that stopped a Job adding a dependency would refuse the work rather than protect anything. The gaming check is the right instrument there — it flags, and a Judge reads.

**The ordinary tier is what a Judge may lift, and the shipped workflows now say only that.** `.env` was in every one of them and is gone from all nine, because an entry sitting in the liftable list teaches the next author that that is where secrets go. What is left is `node_modules/` and `target/`: generated output, which decides nothing about what is checked or judged, and which a Judge asked about would refuse on its own.

## What else the review changed

- **Lifetime is on every setting.** The live-versus-frozen split was two example lists, so any setting on neither had undefined spawn behaviour. Four values: `Live`, `Frozen for the Job`, `Daemon start`, `Read at render`.

  **`Frozen for the Job` was `Frozen at spawn` until Focus**, and the rename is
  the whole of the change — the same fourteen settings, resolved at the same
  moment, said correctly. A Drone belongs to a workflow step, so a Job spawns
  one per step; "frozen at spawn" would have meant re-resolved at every step
  boundary, which is exactly what the snapshot rule forbids. Fleet snapshots
  what a Drone works under when the **Job** is created and hands the same
  snapshot to every step's Drone. `../concepts/drone.md` carries the rule and
  why it is one.
- **`read_by` names the crate that will read a setting at runtime**, which nothing recorded before. The name is deliberate: no v2 code exists yet, so every value is a **design decision, not an observation**, and the column must not be mistaken for a record of fact. It stops being a design decision the day each owning crate declares the keys it reads and this list is generated from them.

That day also closes the one gap it cannot close now. A setting nothing reads is visible — `Helm action authority` and `Helm budget soft-warning threshold` carry `unassigned (no crate)`, because Helm has no crate at all, and six Bridge settings carry `bridge (TS)`, for which no dependency path in the crate graph delivers config. **A crate listed here that reads nothing is not visible**, and cannot be: it is a claim about code that does not exist. Once each owning crate's exhaustive match declares the keys it reads, both directions collapse into one diff.

- **Owner is the unit that enforces a setting, not the page that documents it.** Six settings moved: `Drone/Job timeout`, `Approval request timeout`, `Job retention` and `Drone heartbeat` were filed under Job and Drone, which are records rather than actors — Fleet enforces all four. `Secrets exposure method` moved from Drone to Fleet for the same reason. Where a setting's documenting page is not a Concept row at all, Owner is deliberately empty: both copy-lint settings, because the Agent Copy Contract is a contract rather than a component.
- **Fourteen settings were added that code will read and nothing defined.** The Judge — the only tier in the system that measures correctness — had none of the original settings: no model, no threshold, no cost cap, no trigger set. Neither did `ModelClient`, `AgentHarness`'s binary path, the Secrets provider, the VCS host, the worktree root, the Evidence MCP bind address, or `verification`'s `max_context_size`. Each carries a `default` of `undecided` rather than a silent constant.

## Open questions

- **[evidence-clarification-round-cap]** Is the evidence clarification-round
  cap 2 or 3?
  The cap is a content-sufficiency counter — evidence arrived through the
  Evidence MCP tool but was insufficient, so Fleet prompts for more. It is
  distinct from `poke_limit`, the liveness counter used for silence and
  plain-text bypass, even though both terminate in a `stalled` escalation.
  The value is directionally 2-3 rounds; the exact value is a Kit/Manifest
  config row and needs a shipped default.

- **[root-manifest-default-posture]** What is the schema and surface for the
  root-Manifest default-posture setting?
  The Job-shape classifier leans on this setting as a prior — prefer Convoy
  versus prefer strict per-workspace boundaries. Its existence is agreed;
  its schema and surface are not. Open: whether it is a two-value toggle or
  a scale; whether it lives in the root `armada.yml` only or a workspace may
  override it; where it is edited — Manifest Setup, or a settings surface
  after the fact; and whether the classifier's proposed shape is shown
  alongside the posture that produced it, so an unexpected proposal is
  diagnosable. The step's Definition of Done requires that changing the
  posture measurably changes the proposed shape for an otherwise-identical
  prompt, which is untestable until the setting has a schema.

- **[commit-template-vs-copy-lint]** Which wins, a Manifest commit/PR
  template or the Agent Copy Contract's lint?
  A Manifest-level commit/PR message template is a per-project convention;
  the lint is a fixed phrase blocklist. A project template could mandate a
  format the lint rejects — a required bulleted body, for example.
  Precedence is undecided: if the lint only warns, the collision is
  cosmetic; if it gates, a project's own convention can block its own
  Drones.

- **[helm-budget-warning-threshold]** What is Helm's budget soft-warning
  threshold value?
  Helm has a soft-warning threshold and deliberately no hard cap — it is a
  human-driven tool steered in real time, not autonomous background spend.
  The config slot exists; the default value does not.

- **[copy-lint-surface-narrowing]** May a Manifest narrow which surfaces the
  Agent Copy Contract's lint covers?
  PR descriptions, commit messages and Judge summaries are linted by
  default; evidence summaries and Helm replies are deliberately excluded.
  The question originally leaned on the config direction rule — narrower or
  off is legal, wider is not — but that rule was withdrawn 22 Aug 2026 (see
  "Polarity is a description, not a legality rule" above), so it no longer
  settles anything here. Whether the linted set should be narrowable by a
  Manifest at all is the live question, given that PR and commit text
  leaves the app permanently once written. A parallel question — whether a
  Voice setting can widen or narrow the same lint — is tracked separately in
  Notion.

- **[judge-prompt-assembly]** Does verification assemble the Judge prompt,
  or does Fleet assemble it and verification return structured verdicts
  that Fleet renders?
  Raised by the config review, deciding whether Voice/tone is an input to
  verification — Voice governs runtime-generated prose, including Judge
  summaries. If verification assembles the prompt, it needs Voice, and the
  settings row listing it as a consumer is right. If Fleet assembles the
  prompt and verification returns structured verdicts, verification never
  sees Voice, and that row names a consumer that does not exist. The second
  shape is the cleaner one and keeps verification free of anything but a
  `ModelClient` call, so it is the likelier answer. It also determines
  whether the Agent Copy Contract's lint gates a string verification
  produced or one Fleet produced.

- **[features-and-limits-settings]** What are the Features and Limits
  settings?
  Named in the original concept notes as Guild settings and never defined —
  too vague to become config rows at the time, and still undefined. They
  either resolve into concrete rows here, or get dropped explicitly; left as
  a name with no definition, they quietly evaporate. Whether they are Kit or
  Machine settings cannot be assigned until the settings themselves are
  defined — the project-level test has nothing to run against.

- **[merge-strategy-lifetime-enforcement]** Should merge strategy and
  lifetime be enforced structurally rather than left as documented
  convention, and if so, when does that land?
  Both properties exist as columns in the settings list, and every row
  carries a value today, but nothing prevents a future row leaving either
  blank — every v1 failure was a convention failure. The structural
  alternative was costed and not taken: a `Setting` trait with no-default
  `MERGE` and `LIFETIME` associated constants, polarity newtypes (`Cap`
  merges by min, `Floor` by max, an allow-shaped list by intersection, a
  deny-shaped list by union) so `REPLACE` is not constructible for a cap, a
  `SettingKey` enum with one exhaustive match per owning crate, and two
  golden tests — a registry-key test against the serde field names of
  `KitConfig` and `ManifestConfig` under `deny_unknown_fields`, and a
  snapshot of the descriptor list so a tier move is a reviewable diff.
  Roughly two days, about ten lines per new setting. Deferred to the commit
  that first writes a merge function, because a merge function written
  without polarity is the bug this review was about. Left open: the
  registry-key test only has two halves — `KitConfig`, `ManifestConfig` — a
  Machine setting has no corresponding half, so whether a third half exists
  and what it is called is config-source-enum-values, below.


- **[config-source-enum-values]** What are the legal values of the `source`
  tag Fleet writes on each key of a Job's resolved config record?
  The record is a flat map of key → (value, source), written once per Job
  at spawn, with `source` recorded as `kit` / `machine` / `manifest` /
  `default` today — `guild.yml` itself is retired, replaced by `kit.yml`
  and `machine.yml`. Whether Machine participates in the resolution
  `get_manifest` returns, or sits outside it as per-install settings a
  Manifest never sees, is itself undecided and gates this question — it
  cannot be answered ahead of that one. Machine has one value, no Manifest
  tier and no merge, so a Machine-sourced value can never have been
  overridden — whether `machine` is a fourth source value or a
  non-participant that never appears in the record is the crux. The
  tier-moves argument for hard-failing unknown keys was worked through
  Budget moving from one tier to a two-tier scope; Budget is Machine, and
  Machine has no second tier, so that move is not possible. The example has
  since been replaced, and the argument survives without it — see "An
  unknown key in `armada.yml` hard-fails" above. The registry-key golden test checks keys against the serde field
  names of `KitConfig` and `ManifestConfig` under `deny_unknown_fields`; a
  Machine setting has no corresponding half, so an orphan on the Machine
  side goes uncaught.

- **[armada-yml-schema]** What is the exact `armada.yml` schema?
  M1 decided a subset — `version`, `id`, `base`, `checks.<name>.run`,
  `checks.<name>.when`, `checks.<name>.requires`, `commands.<name>.run`,
  `commands.<name>.destructive` and `setup.requires` — with every
  other key (`permissions`, `knowledge`, `policy`,
  `commands.*.description`) hard-failing as unknown until Reach. Beyond that, what is still open: the
  file syntax for how a Check declares its command; how the Commands
  registry is structured and relates to the allowlist; how Checks and
  Commands are told apart on disk; the syntax for a root Check's path
  condition (its meaning — root paths are the paths no workspace claims —
  is already settled); and how a Manifest declares its own values for
  two-tier Kit settings. A design pass proposes a concrete schema —
  sections for `version`, `id`, `checks`, `commands`, `setup`,
  `permissions`, `knowledge` and `policy`, with `permissions` intersecting
  across a Convoy and `knowledge` unioning — tested against the Armada Job
  Scenarios, but is explicitly not a decision: scenarios it cannot express
  (a Workspace created mid-Job becoming a gate; a workspace depending on a
  sibling workspace's output) are named, and further gaps are filed as
  their own open items, including a Drone's ability to weaken its own gate
  by editing the file that defines it, and no tombstone for a deleted
  Manifest's `id`. Position validation, the VCS-root walk, one schema
  validated by position, exit-code-only Checks, and Commands as their own
  ungated grant are already decided and not open for re-litigation.

- **[deliverable-location]** Should a step's deliverable go on living inside
  `.armada/`, or somewhere outside it?
  `.armada` is an absolute boundary — the tier no Judge can lift and no key can
  widen, because it holds the workflow definitions a step is judged by. Seven
  shipped workflows nonetheless send a step's deliverable to
  `.armada/artifacts/`, by a `mechanical_checks[].target` a Drone did not choose
  and cannot move, and Fleet opens exactly that path at the gate to build the
  Judge's brief. `#431` carved that directory out of the boundary, compiled in
  beside it, so the deliverable works and the tier still has no key.
  What decides it: whether a boundary with a hole in it is the shape wanted, or
  whether a deliverable simply does not belong under a directory an agent is
  otherwise forbidden. The carve-out is the smaller change and is in force; the
  alternative is moving deliverables out of `.armada` entirely, which is a
  workflow-schema change touching all seven and every repository's expectations
  of where its artifacts land. Latent until now only because this repository
  gitignores `.armada/*`, so an artifact never entered a diff and the gate never
  saw one — a repository that does not ignore it meets this on its first `plan`
  step.

Also bearing on this document, and written where each belongs: `[adapter-admission-test]` in `adapters.md`. A question has one home — answering it in two places is how one of them goes stale.
