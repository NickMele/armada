//! What a Judge call is, as a value.
//!
//! No process, the same as the harness cases beside these: whether a call
//! carries a session, a toolset or a directory is a question about a rendering.

use adapter_traits::{Ask, Environment, Model, ModelClient};

use crate::HeadlessAgent;

fn ask() -> Ask {
    Ask::put(
        Model::named("the-cheap-model").expect("a model name"),
        "Does the fix address the cause the note names?",
        Environment::nothing()
            .and("PATH", "/usr/bin:/bin")
            .expect("a legal variable"),
    )
    .expect("a legal ask")
}

fn arg_after(args: &[String], flag: &str) -> Option<String> {
    let at = args.iter().position(|arg| arg == flag)?;
    args.get(at + 1).cloned()
}

/// The one-shot properties, each as a flag on the list. A Judge that could take
/// a second turn could go looking, and a verifier that goes looking is not
/// reproducible.
#[test]
fn a_judge_call_takes_one_turn_and_holds_no_tool() {
    let call = HeadlessAgent::on_path().render(&ask());
    let args = call.args();
    assert_eq!(arg_after(args, "--max-turns").as_deref(), Some("1"));
    assert_eq!(arg_after(args, "--allowedTools").as_deref(), Some(""));
    assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
    // Strict with no configuration beside it is the empty set. A Judge does not
    // even hold the Evidence tool a Drone always has.
    assert!(!args.iter().any(|arg| arg == "--mcp-config"));
}

/// It is not a session. A Drone's stdin carries one JSON object per line for
/// the life of the Job; this one carries a question and then closes.
#[test]
fn a_judge_call_is_not_a_session() {
    let call = HeadlessAgent::on_path().render(&ask());
    assert!(!call.args().iter().any(|arg| arg == "--input-format"));
    assert!(!call
        .args()
        .iter()
        .any(|arg| arg == "--replay-user-messages"));
}

/// Nothing readable in argv, for the reason a Drone's prompt is not there: `ps`
/// prints a same-uid child's argument list. A criterion quotes the work.
#[test]
fn the_question_is_on_stdin_and_not_on_the_argument_list() {
    let call = HeadlessAgent::on_path().render(&ask());
    assert_eq!(
        call.question(),
        "Does the fix address the cause the note names?"
    );
    assert!(
        !call.args().iter().any(|arg| arg.contains("the note names")),
        "{:?}",
        call.args()
    );
}

/// **There is no field for a worktree**, so a Judge cannot be pointed at one.
/// The environment is the caller's and the adapter cannot substitute another.
#[test]
fn a_judge_call_names_no_directory_and_carries_the_environment_it_was_given() {
    let call = HeadlessAgent::on_path().render(&ask());
    assert_eq!(call.environment().names(), vec!["PATH"]);
    assert!(
        !call.args().iter().any(|arg| arg.contains("worktree")),
        "{:?}",
        call.args()
    );
}

/// The per-step dial is what arrives; the default is what it moves away from.
#[test]
fn the_model_on_the_list_is_the_one_the_ask_named() {
    let call = HeadlessAgent::on_path().render(&ask());
    assert_eq!(
        arg_after(call.args(), "--model").as_deref(),
        Some("the-cheap-model")
    );
    assert!(HeadlessAgent::models().contains(&HeadlessAgent::judge_model()));
}

/// The default is `crates/config/settings.toml`'s decision, and this is where
/// the two are held to each other.
///
/// **The derivation is what needs pinning, not the string.** `judge_model` does
/// not write the value down — it takes the last entry of the roster, on the
/// assumption that the roster runs strongest first. That assumption is about a
/// list this module does not own, so a reordered or extended roster would
/// redecide a setting without anybody deciding anything. Here it costs a red
/// test instead, which is the point at which somebody either updates the row or
/// puts the order back.
#[test]
fn the_default_judge_model_is_the_value_the_settings_row_carries() {
    assert_eq!(HeadlessAgent::judge_model(), "haiku");
    // Cheapest, not merely present: it is the end of the roster rather than
    // somewhere in the middle of it.
    assert_eq!(
        HeadlessAgent::models().last().copied(),
        Some(HeadlessAgent::judge_model())
    );
    // And below the model a Job itself gets. A Judge that cost what the work it
    // checks costs would be a second Drone rather than a veto.
    assert_ne!(HeadlessAgent::judge_model(), HeadlessAgent::default_model());
}
