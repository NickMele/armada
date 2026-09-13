//! The apparatus Reach's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for the reason `board.rs` and `landing.rs` are: M1's
//! bench answers "did this Job pass its gates" against Armada's own workflow,
//! and Reach asks what holds for a repository that is not Armada's at all.
//!
//! **Two documents and a path, held as text.** Nothing here is read off disk,
//! so the repository below exists only as the files it would have: the
//! `armada.yml` Setup would end in, and a workflow definition written without
//! that repository in view. Both go through the parsers Fleet loads with, so a
//! fixture no Fleet would load is refused here rather than asserted against.

use std::path::Path;

use config::{Manifest, ResolveError, ResolvedWorkflow, Roster, WorkflowDef};

/// Where the repository's Manifest would be. Absolute, and not this
/// repository: a web shop on `pnpm`, which shares no Check name with Armada.
pub const MANIFEST_AT: &str = "/repos/storefront/armada.yml";

/// Where a definition Armada carried would say it was read from. A name for
/// the refusals to cite, not a file — and deliberately outside the repository,
/// because a definition from inside it is the one source that works today.
pub const DEFINITION_AT: &str = "/carried/bug.json";

/// The `armada.yml` a finished Setup would write for that repository.
///
/// **Every band the journey's proposal sheet draws, as the file spells it**:
/// a port declared first, three Checks in the order the gate starts them, the
/// Commands one Check requires — the journey's own `e2e` needing `migrate` and
/// `seed` — a destructive Command, a server, and what setup runs.
pub const WRITTEN: &str = r#"
version: 1
id: storefront
ports:
  web:
    container: 3000
    env: PORT
checks:
  test:
    run: pnpm vitest run
  lint:
    run: pnpm eslint .
  e2e:
    run: pnpm playwright test
    requires:
      - migrate
      - seed
commands:
  install:
    run: pnpm install --frozen-lockfile
  migrate:
    run: pnpm prisma migrate deploy
  seed:
    run: pnpm tsx scripts/seed.ts
  reset:
    run: pnpm prisma migrate reset --force
    destructive: true
  dev:
    serve: pnpm next dev -p ${port.web}
    ready: curl -sf http://localhost:${port.web}
setup:
  requires:
    - install
"#;

/// A proposal that says more than the file can hold, four ways at once.
///
/// **Each is a guess the journey's evidence rule exists to stop**: a section
/// copied from `package.json` that nothing reads, a Check put in front of
/// another Check, one script placed in both registries, and setup waiting on a
/// server that never exits.
pub const OVERREACHING: &str = r#"
version: 1
id: storefront
scripts:
  build: pnpm next build
checks:
  test:
    run: pnpm vitest run
  lint:
    run: pnpm eslint .
    requires:
      - test
commands:
  lint:
    run: pnpm eslint . --fix
  dev:
    serve: pnpm next dev
setup:
  requires:
    - dev
"#;

/// A Bug workflow **written without the repository in view**, in the shape the
/// shipped definitions have now — `evidence: { submitted: { type } }`.
///
/// Its gate asks for `every_manifest_check` rather than a name, and its
/// implementing step asks to be captured, which a repository with no
/// `evidence:` section has never opted in to. No step names a model, so the
/// roster it is read against offers none.
pub const CARRYABLE: &str = r#"{
  "version": 1,
  "workflow_id": "bug",
  "name": "bug",
  "structure": "linear",
  "steps": [
    {
      "id": "plan",
      "label": "Plan the change",
      "evidence": { "submitted": { "type": "facts_note" } },
      "mechanical_checks": [
        { "type": "artifact_exists", "target": ".armada/artifacts/plan.md" }
      ],
      "delivers": false,
      "advance_gate": "auto"
    },
    {
      "id": "implement",
      "label": "Implement",
      "evidence": { "submitted": { "type": "diff" }, "captured": true },
      "mechanical_checks": [
        { "type": "every_manifest_check" },
        { "type": "diff_nonempty" }
      ],
      "delivers": false,
      "advance_gate": "auto"
    },
    {
      "id": "handoff",
      "label": "Summarise",
      "evidence": { "submitted": { "type": "facts_note" } },
      "delivers": true,
      "advance_gate": "human_always"
    }
  ]
}"#;

/// The same step gating on Armada's own two Checks **by name** — the way a
/// definition written for one repository reads when it is carried to another.
pub const NAMING_ARMADAS_CHECKS: &str = r#"{
  "version": 1,
  "workflow_id": "bug",
  "name": "bug",
  "structure": "linear",
  "steps": [
    {
      "id": "implement",
      "label": "Implement",
      "evidence": { "submitted": { "type": "diff" } },
      "mechanical_checks": [
        { "type": "manifest_check", "check": "build", "expect_exit_code": 0 },
        { "type": "manifest_check", "check": "test", "expect_exit_code": 0 }
      ],
      "delivers": false,
      "advance_gate": "auto"
    }
  ]
}"#;

/// The written Manifest, loaded. Panics on a refusal, which the claim asserts
/// separately and first.
pub fn written() -> Manifest {
    Manifest::parse(Path::new(MANIFEST_AT), WRITTEN)
        .unwrap_or_else(|why| panic!("the written Manifest loads: {why}"))
}

/// A definition, parsed against a roster offering nothing — none of the ones
/// here names a model.
pub fn definition(text: &str) -> WorkflowDef {
    WorkflowDef::parse(Path::new(DEFINITION_AT), text, &Roster::offering_nothing())
        .unwrap_or_else(|why| panic!("the definition parses: {why}"))
}

/// A definition resolved against the repository's own Manifest, as Fleet
/// resolves one at start.
pub fn resolved_there(text: &str) -> Result<ResolvedWorkflow, ResolveError> {
    ResolvedWorkflow::resolve(&definition(text), &written())
}
