//! The apparatus Reach's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for the reason `board.rs` and `landing.rs` are: M1's
//! bench answers "did this Job pass its gates" against Armada's own workflow,
//! and Reach asks what holds for a repository that is not Armada's at all.
//!
//! **Held as text.** Nothing here is read off disk, so the repository below
//! exists only as the files it would have: the ones Scan reads before anybody
//! set it up, the `armada.yml` Setup would end in, and a workflow definition
//! written without that repository in view. Each goes through what Fleet reads
//! it with, so a fixture no Fleet would read is refused rather than asserted.

use std::collections::BTreeMap;
use std::path::Path;

use config::{
    Catalogue, Fault, Manifest, ResolveError, ResolvedCatalogue, ResolvedWorkflow, Roster,
    WorkflowDef, Written,
};
use fleet::scanning::{Entry, Read, Tree};

mod setup;
// Only Reach's own test binary reaches Setup's apparatus; the other milestones compile it unused.
#[allow(unused_imports)]
pub use setup::*;

/// The repository's checkout, as a Scan of it would say it read.
pub const CHECKOUT: &str = "/repos/storefront";

/// The storefront **before its Manifest was written**, as the files Scan finds:
/// a `pnpm` workspace, a compose file, and a hidden directory of YAML.
///
/// `packages/tokens` names nothing runnable and `services/mailer` is on a tool
/// Scan does not follow, so all three strengths are here to tick by.
pub const UNSET_UP: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"storefront","private":true,"scripts":{"lint":"pnpm -r lint"}}"#,
    ),
    ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'apps/*'\n  - 'packages/*'\n",
    ),
    (
        "compose.yaml",
        "services:\n  db:\n    image: postgres:16\n    ports:\n      - \"5432:5432\"\n",
    ),
    (".ci/pipeline.yml", "steps:\n  - run: pnpm test\n"),
    (
        "apps/shop/package.json",
        r#"{"scripts":{"test":"vitest run","lint":"eslint .","e2e":"playwright test","dev":"next dev --port 3000"}}"#,
    ),
    (
        "apps/admin/package.json",
        r#"{"scripts":{"test":"vitest run","lint":"eslint ."}}"#,
    ),
    (
        "packages/ui/package.json",
        r#"{"scripts":{"test":"vitest run"}}"#,
    ),
    (
        "packages/tokens/package.json",
        r#"{"name":"@storefront/tokens"}"#,
    ),
    ("services/mailer/go.mod", "module storefront/mailer\n"),
];

/// A repository held in memory, as a Scan's [`Tree`]. Reading is all it
/// offers, because reading is all a `Tree` is.
pub struct Held(BTreeMap<String, String>);

impl Held {
    pub fn of(files: &[(&str, &str)]) -> Held {
        Held(
            files
                .iter()
                .map(|(path, text)| (path.to_string(), text.to_string()))
                .collect(),
        )
    }

    pub fn has(&self, path: &str) -> bool {
        self.0.contains_key(path)
    }
}

impl Tree for Held {
    fn read(&self, path: &str) -> Read {
        match self.0.get(path) {
            Some(text) => Read::Bytes(text.as_bytes().to_vec()),
            None => Read::Absent,
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<Entry>, String> {
        let prefix = match dir.is_empty() {
            true => String::new(),
            false => format!("{dir}/"),
        };
        let mut found: BTreeMap<String, bool> = BTreeMap::new();
        for path in self.0.keys() {
            let Some(rest) = path.strip_prefix(&prefix) else {
                continue;
            };
            match rest.split_once('/') {
                Some((child, _)) => found.insert(child.to_string(), true),
                None => found.insert(rest.to_string(), false),
            };
        }
        if found.is_empty() && !dir.is_empty() {
            return Err(format!("{dir} is not a directory here"));
        }
        Ok(found
            .into_iter()
            .map(|(name, is_dir)| Entry { name, is_dir })
            .collect())
    }
}

/// Where the repository's Manifest would be. Absolute, and not this
/// repository: a web shop on `pnpm`, which shares no Check name with Armada.
pub const MANIFEST_AT: &str = "/repos/storefront/armada.yml";

/// Where a definition written without the repository in view says it was read
/// from. A name for the refusals to cite, not a file.
pub const DEFINITION_AT: &str = "/carried/bug.json";

/// Where a person's Kit keeps its Workflows, on a machine that is not this one.
pub const KIT_AT: &str = "/home/user/.armada/workflows";

/// Where the storefront keeps its own, beside [`MANIFEST_AT`].
pub const OWN_AT: &str = "/repos/storefront/.armada/workflows";

/// A definition of one ungated step. What a person writes to replace a carried
/// workflow without caring what it did.
pub fn one_step(id: &str) -> String {
    format!(
        "version: 1\nworkflow_id: {id}\nname: {id}\nstructure: linear\nsteps:\n  - id: only\n    \
         label: Only\n    delivers: true\n    advance_gate: auto\n"
    )
}

/// What Armada carries, merged with what a Kit and the repository wrote, as
/// `(file, text)`, and resolved against the storefront. Read against a roster
/// offering exactly the models the carried set names — [`carried_there`] says
/// why the roster is not this claim.
pub fn catalogued(kit: &[(&str, String)], own: &[(&str, String)]) -> ResolvedCatalogue {
    let named: Vec<String> = config::carried()
        .iter()
        .filter_map(|one| {
            WorkflowDef::parse(one.path(), one.text(), &Roster::offering_nothing()).err()
        })
        .flat_map(|refused| {
            refused
                .refusals()
                .iter()
                .filter_map(|refusal| match &refusal.fault {
                    Fault::NoSuchModel { value, .. } => Some(value.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let at = |dir: &str, file: &str| Path::new(dir).join(file);
    let written = config::carried()
        .into_iter()
        .chain(
            kit.iter()
                .map(|(file, text)| Written::in_kit(at(KIT_AT, file), text.clone())),
        )
        .chain(
            own.iter()
                .map(|(file, text)| Written::in_repository(at(OWN_AT, file), text.clone())),
        );
    Catalogue::of(written, &Roster::of(named))
        .unwrap_or_else(|why| panic!("the three places merge: {why:?}"))
        .resolve(&self::written())
        .unwrap_or_else(|why| panic!("nothing the storefront wrote is refused: {why}"))
}

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

/// [`WRITTEN`] as a person keeps it after Setup wrote it: the same keys, with
/// comments and spacing of their own — and `lint` on a command Verify failed.
pub const KEPT: &str = r#"# The storefront's Manifest, kept by hand since Setup wrote it.
version: 1
id: storefront

# Armada places the number; the app reads it as PORT.
ports:
  web:
    container: 3000
    env: PORT

# In the order the gate starts them.
checks:
  test:
    run: pnpm vitest run
  # Failed Verify: this repository lints per package, not from the root.
  lint:
    run: pnpm eslint .
  e2e:
    run: pnpm playwright test
    # `migrate` before `seed`: seed writes into the tables migrate made.
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
  # Drops the database. A Drone asks first.
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

/// **Armada's own `epic`, as the file ships** — the workflow whose subject is
/// running a milestone.
///
/// Held by `include_str!`, so it is compiled in rather than read at run time and
/// the test still touches no file. The real file rather than a fixture shaped
/// like it: a fixture would go on saying what `epic` is for after `epic.json`
/// had stopped.
pub const EPIC: &str = include_str!("../../../../.armada/workflows/epic.json");

/// Where a carried `epic` would say it was read from. See [`DEFINITION_AT`].
pub const EPIC_AT: &str = "/carried/epic.json";

/// What a person types when the work is a milestone. **No word of any `epic`
/// step label is in it**, which is the case #424 was filed on.
pub const A_MILESTONE: &str = "Finish the Board milestone, every issue still open under it";

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

/// A shipped definition resolved against the repository's own Manifest, read
/// against a roster offering **exactly the models it names**.
///
/// The roster is not this claim: whether a shipped model is one this machine
/// can run is `crates/config/tests/shipped.rs`, against the adapter's own list.
/// Spelling that list here would put a vendor's aliases in a crate gate rule six
/// keeps them out of, so the first read's `NoSuchModel` refusals are the roster
/// for the second — and any other refusal still fails the second read.
pub fn carried_there(path: &str, text: &str) -> ResolvedWorkflow {
    let named: Vec<String> =
        match WorkflowDef::parse(Path::new(path), text, &Roster::offering_nothing()) {
            Ok(def) => return resolved_def(&def),
            Err(refused) => refused
                .refusals()
                .iter()
                .filter_map(|refusal| match &refusal.fault {
                    Fault::NoSuchModel { value, .. } => Some(value.clone()),
                    _ => None,
                })
                .collect(),
        };
    let def = WorkflowDef::parse(Path::new(path), text, &Roster::of(named))
        .unwrap_or_else(|why| panic!("{path} parses: {why}"));
    resolved_def(&def)
}

fn resolved_def(def: &WorkflowDef) -> ResolvedWorkflow {
    ResolvedWorkflow::resolve(def, &written())
        .unwrap_or_else(|why| panic!("{} resolves there: {why}", def.path().display()))
}

/// The workflow a catalogue resolved to for `id`, against the storefront.
pub fn held_there(catalogue: &ResolvedCatalogue, id: &str) -> ResolvedWorkflow {
    catalogue
        .workflows()
        .values()
        .find(|workflow| workflow.id().as_str() == id)
        .unwrap_or_else(|| panic!("`{id}` is held"))
        .clone()
}
