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

The decisive argument is tier moves. If unknown keys were ignored, a `budget:` key written today would do nothing — and the day **Budget: $ cost cap per Job/Drone** moves from `Machine` to `Kit → Manifest`, it would wake up and change behaviour on a repo nobody had touched. Under hard fail that key could never have been written, so the tier move is safe by construction.

The cost is the collision this project has already paid once: a file written by one module and parsed by another with `deny_unknown_fields`. That is precisely why the answer is namespacing per owner rather than handing the file a single parser.

## What else the review changed

- **Lifetime is on every setting.** The live-versus-frozen split was two example lists, so any setting on neither had undefined spawn behaviour. Four values: `Live`, `Frozen at spawn`, `Daemon start`, `Read at render`.
- **`read_by` names the crate that will read a setting at runtime**, which nothing recorded before. The name is deliberate: no v2 code exists yet, so every value is a **design decision, not an observation**, and the column must not be mistaken for a record of fact. It stops being a design decision the day each owning crate declares the keys it reads and this list is generated from them.

That day also closes the one gap it cannot close now. A setting nothing reads is visible — `Helm action authority` and `Helm budget soft-warning threshold` carry `unassigned (no crate)`, because Helm has no crate at all, and six Bridge settings carry `bridge (TS)`, for which no dependency path in the crate graph delivers config. **A crate listed here that reads nothing is not visible**, and cannot be: it is a claim about code that does not exist. Once each owning crate's exhaustive match declares the keys it reads, both directions collapse into one diff.

- **Owner is the unit that enforces a setting, not the page that documents it.** Six settings moved: `Drone/Job timeout`, `Approval request timeout`, `Job retention` and `Drone heartbeat` were filed under Job and Drone, which are records rather than actors — Fleet enforces all four. `Secrets exposure method` moved from Drone to Fleet for the same reason. Where a setting's documenting page is not a Concept row at all, Owner is deliberately empty: both copy-lint settings, because the Agent Copy Contract is a contract rather than a component.
- **Fourteen settings were added that code will read and nothing defined.** The Judge — the only tier in the system that measures correctness — had none of the original settings: no model, no threshold, no cost cap, no trigger set. Neither did `ModelClient`, `AgentHarness`'s binary path, the Secrets provider, the VCS host, the worktree root, the Evidence MCP bind address, or `verification`'s `max_context_size`. Each carries a `default` of `undecided` rather than a silent constant.
