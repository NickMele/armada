# Adapters

**Kind:** spec. **Governs:** the adapter boundaries — AgentHarness, VCS,
Secrets, ModelClient, and Docker, added Aug 2026 and unscoped — what each
may expose, and where adapter configuration lives.

---

## The adapter rule

**An adapter is code Armada calls deterministically.** Armada decides when
it runs, with what arguments, and what to do with the answer. Skills, MCP
servers and Plugins are tools *the Drone chooses to call* during its own
reasoning — a separate axis, and one that stays in Kit and Manifest. The
distinction is not about the technology; it is about who decides. A model
call Armada makes to judge evidence is an adapter. The same model reached
through a tool the Drone elects to invoke is not.

**Secrets is Fleet-side only.** A Drone never holds a callable secrets
tool. Fleet resolves and injects; the Drone receives values, never the
means to ask for more.

## The four adapters — plus a fifth, added Aug 21 2026

> **`Docker` is a fifth adapter, and it was taken unscoped.** Decided when
> the port-allocation design asked how a compose stack receives its claimed
> ports: Armada resolves the repo's compose files, rewrites every published
> port into the claimed span, and feeds the whole document on stdin. See
> the open question on how a compose stack receives its claimed ports, in
> Armada Decisions.
>
> **It satisfies the adapter rule.** Armada decides when it runs, with what
> arguments, and what to do with the answer. The Drone never calls it.
>
> **The alternative bought the boundary and lost the rule.** Env
> interpolation — the repo writes `${ARMADA_PORT_WEB:-3000}` and Armada
> never parses the document — needed no adapter and gave up v1's strongest
> single rule: **refuse any published port Armada did not place**. Its
> mitigation was a Setup proposal and a Verify drift warning, which are
> warnings where v1 had a refusal.
>
> **Boundary cost, stated rather than assumed.** A narrowly-scoped
> compose-rewrite adapter — one operation, no lifecycle, no state, no
> health model — was considered and **not** taken. Four adapters was a
> chosen boundary; five is a precedent for six, and nothing in that
> decision says where this one stops. Docker also already appears as a
> Doctor health probe, so this makes an existing dependency explicit rather
> than introducing one.
>
> Its own configuration rows, structural constraints and open questions
> are not written yet.

| Adapter | What Armada calls it for | Concrete implementation |
| --- | --- | --- |
| **AgentHarness** | Spawn a Drone, parse its structured output, report status | Claude Code CLI, headless mode |
| **VCS** | Fleet-level worktree, branch, push, PR and merge orchestration | Git (git2) + GitHub |
| **Secrets** | Resolve granted secrets before a run detaches, and inject them at spawn | 1Password, env-injection at Drone spawn |
| **ModelClient** | One-shot model calls Armada makes itself | Anthropic API, cheap model |
| **Docker** — added Aug 2026, unscoped | Resolve a repo's compose files, rewrite published ports into a Job's claimed span, run the transformed document on stdin. Also the surface Doctor's existing health probe belongs behind | Docker CLI / Compose. Note the inherited trap: an override **appends** to `ports:` rather than replacing, and the `!override` fix is silently ignored below Compose 2.24.4 — which is why the whole resolved document is transformed in memory rather than layered |

### ModelClient, and what it deliberately does not cover

`ModelClient` was added to the crate design after
[Configuration](configuration.md) was written, which is why it had no rows
anywhere until the August 2026 config review. It has exactly two callers:

- **the Judge Check** — cheap-model semantic verification of whether
  evidence satisfies step intent
- **the Job-shape classifier** — deciding what shape of Job a request is

Both are **one-shot, no tools, cheap model, tight latency budget**. That
shape is the whole point: it is what lets `verification` call a model
without becoming an agent runtime.

**Helm is not a `ModelClient` caller.** Helm is a persistent multi-turn
session with tool access, which is structurally an `AgentHarness` concern,
not a one-shot completion. Putting Helm behind `ModelClient` would force
conversation state, tool dispatch and streaming into a trait whose entire
value is that it has none of those.

## Where adapter config lives

**With the adapter, not in the central schema.** This is the reason this
document exists as a spec.

The tension it resolves: adapter traits exist so that adding an
implementation does not touch core. If a new secrets provider's vault
reference, endpoint and account name had to be added to the central Guild
schema, adding an adapter *would* touch core, and the trait would have
bought nothing.

The resolution is that **the central schema holds exactly one key per
adapter — which provider — and nothing else.**

- `adapter-traits` declares the trait *and* an associated config type
  alongside it. The trait, not a central schema, says what its
  implementations need.
- Each implementation in `adapters` owns its own config struct and
  deserializes its own block.
- **`fleet-bin`, the composition root, is the only place that names a
  concrete implementation.** It reads the provider tag, matches on it, and
  hands the remainder of that block to the implementation to parse.

Everything above `fleet-bin` — `core-model`, `config`, `verification`,
`checks-runner`, `fleet` — sees a trait object and never learns which
provider is behind it.

### Adding a second `Secrets` implementation

| Step | Where |
| --- | --- |
| A module implementing `Secrets`, with its own config struct | `adapters` |
| One new arm in the provider match | `fleet-bin` |
| One new legal value on **Secrets provider selection** | Guild schema |

Nothing in `core-model`, `config`, `verification` or `fleet` changes.
**That is acceptable**, and the reason is worth stating rather than
assuming: the adapter rule promises no *core* change, not no change
anywhere. The composition root is precisely the layer that is allowed to
know every concrete type — naming implementations is its only job. A
design where even `fleet-bin` did not change would require runtime plugin
loading, which for a single-user single-binary system buys nothing and
costs a dynamic-dispatch boundary nobody can typecheck.

### Adding a second `AgentHarness` implementation

**This one is not cheap, and this document should say so plainly.**

`argv` is the permission model. A Drone's entire capability — what it may
run unattended, what is refused however broadly the allowlist grants,
whether MCP config is strict — is granted and withheld through spawn
flags. A second harness must re-express every one of those in its own
vocabulary, and the failure mode when it does not is the worst one in the
system: **a missing capability does not fail, it waits.** A rejected argv
dies in a second with a usage error. A Drone that was never granted
permission burns its entire wall-clock ceiling and then reports a timeout,
which is a symptom of nothing.

So the cost of a second `AgentHarness` is not a module and a match arm. It
is: re-deriving the permission posture, and proving it — which is why the
structural constraint below exists.

## Structural constraints, already decided

These live here now; they were previously scattered across other pages.

- **`AgentHarness` exposes no raw argv builder.** It takes a typed spawn
  config whose MCP and strict-mode fields are **non-optional**, and it has
  no escape-hatch constructor. A second implementation therefore does not
  compile until it has answered every capability question the first one
  answers. This is the structural version of "a rule existed and the code
  didn't enforce it" — the rule is now a field you cannot leave out.
- **The Drone-facing `VCS` type has no `push` method at all.** Not a
  runtime check, not a permission flag: the method is absent from the
  type. A Drone commits locally, inside its own worktree. Fleet performs
  push, PR and merge itself, using credentials Fleet holds directly. A
  capability that does not exist on the type cannot be reached by a Drone
  that reasons its way around a denial.
- **`Secrets` returns `Secret<T>`, which implements no `Debug`, no
  `Display`, and no `Serialize`.** A secret cannot reach a log line, a
  transcript, an Evidence payload or the wire, because there is no code
  path that can render it.
- **The raw diff-computation method is reachable only from
  `verification`.** Diffing is how scope violations are detected; exposing
  it more widely would let a caller compute a diff without the
  verification decisions attached to it.

## Where the traits live, and why the split matters

**Trait definitions live in `adapter-traits`, which has near-zero
dependencies. Implementations live in `adapters`.**

That split is what lets `verification` call a model without pulling in an
HTTP client. `verification` depends on `adapter-traits`; it names
`ModelClient` and never sees `reqwest`, TLS, retry policy or a connection
pool. The same split keeps `testkit` cheap: fakes implement the traits
with no transport at all, which is what makes NDJSON fixtures a complete
substitute for a live harness in tests.

## Configuration rows

Every adapter setting now has a row in the Armada Configuration Settings
database, filtered to `Should be read by = adapters`. Before the August
2026 config review, adapters had no rows at all — which meant the binary
path, the endpoint, the provider and the vault reference were all destined
to become hardcoded constants or environment variables read at the call
site, invisible to Guild, to Manifest and to Doctor.

## Still open

- **Notifications is a lower-priority adapter candidate.** In-app only
  today; OS, Slack and others are plausible later. It stays a config value
  rather than a full adapter until there is a second real target — the
  trait is worth nothing with one implementation.
- **Whether `Secrets provider selection` should ever become `Guild →
  Manifest`.** It is Guild-only today on the tiering rule (a machine
  constraint). A repo that needs a different vault is the case that would
  force the move, and a tier move is expensive. `peer_polarity` in
  `crates/config/settings.toml` is where that lives now — see
  `configuration.md` for why the field was renamed from `Narrowing rule` when
  the config direction rule was withdrawn.

Two embedded lists closed the original and are not reproduced here. One was
open questions from the decision record, filtered to this subject; the ones
still open are below. The other was the settings list, which is now
`crates/config/settings.toml` — the crate that reads them owns it, and
`configuration.md` holds the rules they obey.

## Open questions

- **[adapter-admission-test]** What test admits something as an Armada
  adapter?
  The compose decision added Docker as a fifth adapter, taken unscoped — "the
  boundary is left open and this decision records no limit" — with its own
  warning that four adapters was a chosen boundary and five is a precedent
  for six. Configuration still names four adapters, so the fifth is
  undocumented as well as unbounded. An adapter is the seam that keeps the
  daemon core testable with no network; a set that grows on precedent rather
  than on a test erodes that. A narrower, related question — whether an
  agent harness must support MCP to count as an adapter — tests one adapter
  kind, not the seam itself.

- **[platform-differences-layer]** Is there a layer that owns platform
  differences, and what belongs behind it?
  The port-ceiling decision reads the platform's ephemeral floor at runtime
  and records that this is the first place Armada reads a kernel parameter —
  a small but genuinely new platform dependency that belongs behind
  whatever handles platform differences rather than at the call site —
  naming the layer without identifying it. The process-group ownership
  decision notes that process-group semantics differ across platforms, that
  v1 was bitten by both a Linux-specific and a BSD-specific regression, and
  marks itself still open, though that gap appears only in its body, not in
  its resolution. Armada is macOS single-user today; both dependencies are
  cheap now and expensive to retrofit if Linux returns. The existing adapter
  set is a boundary already widened once — see adapter-admission-test above.
