//! `armada manifest skills` — what this repository knows about itself
//! (PLAN.md §4.8).
//!
//! **There is deliberately no way to run one, and that is the point.** "Add a
//! migration" has no deterministic expansion; `pnpm prisma migrate dev
//! --create-only` does, and that already has a verb. A runner would mean Armada
//! choosing arguments on the user's behalf, which is precisely what §5's layer 1
//! refuses to do.
//!
//! So this file has exactly two read paths over one resolved structure — list,
//! and resolve one — and no third that executes. The CLI table, a future MCP
//! response and the generated frontmatter are three renderings of that one
//! structure, which is what stops a skill meaning one thing to a shell caller
//! and another to an agent.
//!
//! **A read verb**, like `status`: it takes no lease, mutates nothing, and its
//! exit code describes the query rather than the repository.

use armada_core::config::ResolvedConfig;
use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::envelope::{Envelope, GrantedCommand, ResolvedSkillView, ResultRow, SkillsData};
use armada_core::error::{ArmadaError, ErrClass, Status};

use crate::app::App;
use crate::verbs::{load_config, Output};

/// List every declared skill, or resolve one.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    show: Option<&str>,
) -> Result<Output, ArmadaError> {
    let (workspace, config) = load_config(app)?;

    let names: Vec<&String> = match show {
        None => config.skills.keys().collect(),
        Some(name) => {
            if !config.skills.contains_key(name) {
                // `bad_invocation` rather than `bad_config`: the config is fine
                // and the name the caller typed is not
                // (`docs/commands/manifest/skills.md`).
                return Err(ArmadaError {
                    class: ErrClass::BadInvocation,
                    r#where: name.to_string(),
                    message: format!("`{name}` is not a skill this repository declares"),
                    next_action: Some(match config.skills.keys().next() {
                        Some(first) => {
                            format!("`armada manifest skills` lists them — `{first}` is one")
                        }
                        None => "this repository declares no `skills:` at all".to_string(),
                    }),
                });
            }
            vec![config.skills.get_key_value(name).expect("just checked").0]
        }
    };

    let skills: Vec<ResolvedSkillView> =
        names.into_iter().map(|name| view(name, &config)).collect();
    let results = skills
        .iter()
        .map(|skill| {
            // **`OK` and never a verdict.** Listing a skill says the repository
            // declares it, not that anything about it passed — whether its
            // references resolve is `armada manifest config verify`'s answer.
            let mut row = ResultRow::new(skill.name.clone(), Status::Ok);
            row.reason = Some(skill.summary.clone());
            row
        })
        .collect();

    Ok(Output::Skills(Box::new(Envelope::ok(
        "skills",
        Some(workspace.id.clone()),
        Status::Ok,
        SkillsData { results, skills },
    ))))
}

/// One skill with its grants expanded to the commands they name.
///
/// **A `uses:` entry that names nothing keeps its row with an empty `cmd`.**
/// Dropping it would make an unresolved grant invisible on the one verb that
/// lists grants, and `config verify` — which is where that is a *failure* — is a
/// separate command a reader may not have run yet.
fn view(name: &str, config: &ResolvedConfig) -> ResolvedSkillView {
    let skill = &config.skills[name];
    ResolvedSkillView {
        name: name.to_string(),
        summary: skill.summary.clone(),
        doc: skill.doc.clone(),
        uses: skill
            .uses
            .iter()
            .map(|used| GrantedCommand {
                name: used.clone(),
                cmd: config
                    .commands
                    .get(used)
                    .map(|command| command.cmd.clone())
                    .unwrap_or_default(),
            })
            .collect(),
        verify: skill.verify.clone(),
        touches: skill.touches.clone(),
    }
}
