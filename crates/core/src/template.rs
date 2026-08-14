//! Templating: four substitutions plus two scoped placeholders, hard cap
//! (PLAN.md §4.4), and the argv split that goes with them.
//!
//! **Everywhere:** `${port.NAME}`, `${files}`, `${component.root}`,
//! `${workspace.id}`. **`${env.NAME}` is legal in `env:` blocks and nowhere
//! else.** `${ref}` belongs to `secret_providers[].cmd` and arrives with
//! secrets in phase 4.
//!
//! **The cap says what char *substitutes*, not what may appear.** Under
//! `shell: true`, `${HOME}` is ordinary shell syntax and char passes it through
//! untouched — banning it would mean char policing a language it explicitly
//! declined to parse. Under argv-split, which is the default, an unrecognised
//! `${…}` is `bad_config`: nothing downstream would ever expand it, so it can
//! only be a typo.
//!
//! **The split happens before substitution, and that is a security property
//! rather than an implementation detail.** `${files}` is the only substitution
//! whose values come from outside the trust boundary — filenames are written by
//! whoever pushed the branch, and `sub/semi;echo INJECTED.py` is a legal POSIX
//! filename that git emits raw. Expanding into a string and splitting afterwards
//! would word-split a filename containing a space and execute one containing a
//! semicolon (`docs/traps.md`). Expanding a token into *n* tokens cannot.

use crate::error::{CharError, ConfigWhere};
use std::collections::BTreeMap;

/// Everything a template may refer to.
///
/// The inherited environment is a map rather than a live lookup because
/// nothing below the entrypoint may call `std::env::var`
/// (`ARCHITECTURE.md` §1.4) — it is captured once, at the top, and passed down
/// like the workspace is.
pub struct Vars<'a> {
    /// `${workspace.id}`.
    pub workspace_id: &'a str,
    /// `${port.NAME}` — workspace-global, so a component may name another's
    /// port.
    pub ports: &'a BTreeMap<String, u16>,
    /// `${component.root}`, when the site has a component and it declares one.
    pub component_root: Option<&'a str>,
    /// `${files}`. `None` means the placeholder is not available at this site
    /// at all — a `commands:` entry, which has no scope to compute — and using
    /// it there is `bad_config` rather than a silently empty expansion.
    pub files: Option<&'a [String]>,
    /// The inherited environment, for `${env.NAME}`.
    pub env: &'a BTreeMap<String, String>,
}

impl<'a> Vars<'a> {
    /// The minimal set: an id, ports, and the inherited environment.
    pub fn new(
        workspace_id: &'a str,
        ports: &'a BTreeMap<String, u16>,
        env: &'a BTreeMap<String, String>,
    ) -> Self {
        Vars {
            workspace_id,
            ports,
            component_root: None,
            files: None,
            env,
        }
    }
}

/// Where a template appears, which decides what is legal in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    /// A command char splits itself, or a selector char evaluates. An
    /// unrecognised `${…}` is `bad_config` — nothing would expand it.
    Argv,
    /// A command handed to `/bin/sh -c`. An unrecognised `${…}` is the shell's
    /// and passes through untouched.
    Shell,
    /// An `env:` value. `${env.NAME}` is legal here and only here.
    EnvValue,
}

/// Substitute one string, leaving `${files}` for the caller to expand.
///
/// Used for `env:` values and `owns:` selectors, where the result is one value
/// rather than a list of arguments.
pub fn substitute(
    input: &str,
    vars: &Vars,
    site: Site,
    at: &ConfigWhere,
) -> Result<String, CharError> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            // An unterminated `${` is not a placeholder at all; leave it as
            // written rather than inventing a syntax error for a literal.
            out.push_str(&rest[start..]);
            return Ok(out);
        };
        let name = &after[..end];
        match expand(name, vars, site, at)? {
            Expansion::Value(value) => out.push_str(&value),
            Expansion::PassThrough => {
                out.push_str("${");
                out.push_str(name);
                out.push('}');
            }
            Expansion::Files => {
                return Err(unexpandable_files(at));
            }
        }
        rest = &after[end + 1..];
    }

    out.push_str(rest);
    Ok(out)
}

enum Expansion {
    Value(String),
    PassThrough,
    /// `${files}` reached a site that produces one string. The caller that can
    /// expand it — the argv path — never asks this function.
    Files,
}

fn expand(name: &str, vars: &Vars, site: Site, at: &ConfigWhere) -> Result<Expansion, CharError> {
    if name == "workspace.id" {
        return Ok(Expansion::Value(vars.workspace_id.to_string()));
    }
    if name == "files" {
        return match vars.files {
            Some(_) => Ok(Expansion::Files),
            None => Err(CharError::bad_config(
                at.clone(),
                "`${files}` is not available here",
                "remove it — a commands: entry runs ad hoc, so char has no file scope to compute",
            )),
        };
    }
    if name == "component.root" {
        return match vars.component_root {
            Some(root) => Ok(Expansion::Value(root.to_string())),
            None => Err(CharError::bad_config(
                at.clone(),
                "`${component.root}` is used where no component root is in scope",
                "set `root:` on the component, or drop the placeholder",
            )),
        };
    }
    if let Some(port) = name.strip_prefix("port.") {
        return match vars.ports.get(port) {
            Some(number) => Ok(Expansion::Value(number.to_string())),
            None => Err(CharError::bad_config(
                at.clone(),
                format!("no port named `{port}` is declared in this workspace"),
                format!(
                    "declare it under a component's `run.ports`, or correct `${{port.{port}}}`"
                ),
            )),
        };
    }
    if let Some(var) = name.strip_prefix("env.") {
        if site != Site::EnvValue {
            return Err(CharError::bad_config(
                at.clone(),
                format!("`${{env.{var}}}` is only legal inside an `env:` block"),
                "move the read into `env:`, where environment composition lives",
            ));
        }
        return match vars.env.get(var) {
            Some(value) => Ok(Expansion::Value(value.clone())),
            // The loud error is the stopping point, and it has to stay one:
            // the moment `${env.X}` has a default operator, the cap this
            // section spends forty lines defending is gone (PLAN.md §4.4).
            None => Err(CharError::bad_config(
                at.clone(),
                format!("`{var}` is not set in the environment"),
                format!("export {var} before running char, or remove the reference"),
            )),
        };
    }

    match site {
        Site::Shell => Ok(Expansion::PassThrough),
        Site::Argv | Site::EnvValue => Err(CharError::bad_config(
            at.clone(),
            format!("`${{{name}}}` is not one of the four substitutions char makes"),
            "char substitutes ${port.NAME}, ${files}, ${component.root} and ${workspace.id}, \
             plus ${env.NAME} inside env: — set `shell: true` if a shell should expand it",
        )),
    }
}

fn unexpandable_files(at: &ConfigWhere) -> CharError {
    CharError::bad_config(
        at.clone(),
        "`${files}` cannot be used here — it expands to a list of arguments",
        "use it in a check's `cmd:`, which char splits into argv",
    )
}

/// Split a command into argv **and then** substitute, expanding `${files}`
/// into one argument per file.
///
/// A token containing `${files}` becomes one token per file; an empty file set
/// removes the token entirely rather than leaving an empty argument, because an
/// empty string argument means "this path" to most tools.
pub fn expand_argv(cmd: &str, vars: &Vars, at: &ConfigWhere) -> Result<Vec<String>, CharError> {
    let tokens = split_argv(cmd, at)?;
    let mut argv = Vec::with_capacity(tokens.len());
    for token in tokens {
        if token.contains("${files}") {
            let files = vars.files.ok_or_else(|| {
                CharError::bad_config(
                    at.clone(),
                    "`${files}` is not available here",
                    "remove it — a commands: entry runs ad hoc, so char has no file scope",
                )
            })?;
            // **Substitute the token first, insert the filename verbatim
            // afterwards.** Inserting and then substituting re-scans char's own
            // placeholder syntax inside text from outside the trust boundary —
            // filenames are written by whoever pushed the branch. A file named
            // `${port.api}.py` would expand to a port number, and one holding
            // any unrecognised `${…}` would fail the check `bad_config`,
            // letting a pushed branch break the very check that would have
            // examined it.
            let around: Vec<String> = token
                .split("${files}")
                .map(|part| substitute(part, vars, Site::Argv, at))
                .collect::<Result<_, _>>()?;
            for file in files {
                argv.push(around.join(file.as_str()));
            }
        } else {
            argv.push(substitute(&token, vars, Site::Argv, at)?);
        }
    }
    Ok(argv)
}

/// Wrap a command for `/bin/sh -c`, substituting char's own placeholders and
/// leaving everything else for the shell.
pub fn shell_argv(cmd: &str, vars: &Vars, at: &ConfigWhere) -> Result<Vec<String>, CharError> {
    let expanded = substitute(cmd, vars, Site::Shell, at)?;
    Ok(vec!["/bin/sh".to_string(), "-c".to_string(), expanded])
}

/// Split on whitespace, honouring single quotes, double quotes and
/// backslashes — the same grammar a POSIX shell uses to build argv, and no
/// more of one.
///
/// It deliberately implements *no* expansion: `$`, `*`, `~`, `|` and `&&` are
/// ordinary characters here. A repo that needs a shell says `shell: true` and
/// gets a real one.
pub fn split_argv(cmd: &str, at: &ConfigWhere) -> Result<Vec<String>, CharError> {
    let mut argv: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut have_token = false;
    let mut chars = cmd.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if have_token {
                    argv.push(std::mem::take(&mut current));
                    have_token = false;
                }
            }
            '\'' => {
                have_token = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(inner) => current.push(inner),
                        None => return Err(unterminated(at, '\'')),
                    }
                }
            }
            '"' => {
                have_token = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(escaped) => current.push(escaped),
                            None => return Err(unterminated(at, '"')),
                        },
                        Some(inner) => current.push(inner),
                        None => return Err(unterminated(at, '"')),
                    }
                }
            }
            '\\' => {
                have_token = true;
                match chars.next() {
                    Some(escaped) => current.push(escaped),
                    None => current.push('\\'),
                }
            }
            other => {
                have_token = true;
                current.push(other);
            }
        }
    }

    if have_token {
        argv.push(current);
    }

    if argv.is_empty() {
        return Err(CharError::bad_config(
            at.clone(),
            "the command is empty",
            "give it something to run",
        ));
    }
    Ok(argv)
}

fn unterminated(at: &ConfigWhere, quote: char) -> CharError {
    CharError::bad_config(
        at.clone(),
        format!("unterminated {quote} quote in the command"),
        "close the quote",
    )
}

/// Quote one argument for safe inclusion in a `/bin/sh -c` string.
///
/// Needed where char appends caller-supplied argv to a `shell: true`
/// `commands:` entry: the passthrough arguments are the *caller's* text, and
/// concatenating them raw would let `char worktrees ';rm -rf /'` mean something
/// the repo never wrote.
pub fn shell_quote(arg: &str) -> String {
    if !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c))
    {
        return arg.to_string();
    }
    format!("'{}'", arg.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> ConfigWhere {
        ConfigWhere::Path {
            file: "armada.yml".into(),
            path: "components.api.checks.lint.cmd".into(),
        }
    }

    fn ports() -> BTreeMap<String, u16> {
        BTreeMap::from([("api".to_string(), 5461u16), ("pg".to_string(), 5460)])
    }

    fn env() -> BTreeMap<String, String> {
        BTreeMap::from([("REGISTRY".to_string(), "registry.example".to_string())])
    }

    #[test]
    fn the_four_substitutions_expand() {
        let ports = ports();
        let env = env();
        let files = vec!["a.py".to_string()];
        let vars = Vars {
            workspace_id: "a3f91c02",
            ports: &ports,
            component_root: Some("services/api"),
            files: Some(&files),
            env: &env,
        };
        assert_eq!(
            substitute("${workspace.id}", &vars, Site::Argv, &at()).unwrap(),
            "a3f91c02"
        );
        assert_eq!(
            substitute("${port.pg}", &vars, Site::Argv, &at()).unwrap(),
            "5460"
        );
        assert_eq!(
            substitute("${component.root}", &vars, Site::Argv, &at()).unwrap(),
            "services/api"
        );
        assert_eq!(
            expand_argv("check ${files}", &vars, &at()).unwrap(),
            vec!["check", "a.py"]
        );
    }

    #[test]
    fn an_unrecognised_placeholder_is_bad_config_under_argv_split() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        let err = substitute("${HOME}/x", &vars, Site::Argv, &at()).unwrap_err();
        assert_eq!(err.class, crate::error::ErrClass::BadConfig);
        assert!(err.next_action.is_some());
    }

    #[test]
    fn the_same_placeholder_passes_through_untouched_under_a_shell() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        assert_eq!(
            substitute("${HOME}/x", &vars, Site::Shell, &at()).unwrap(),
            "${HOME}/x"
        );
    }

    #[test]
    fn char_still_substitutes_its_own_names_under_a_shell() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        assert_eq!(
            substitute("psql -p ${port.pg} $HOME", &vars, Site::Shell, &at()).unwrap(),
            "psql -p 5460 $HOME"
        );
    }

    #[test]
    fn env_reads_are_confined_to_env_blocks() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        assert_eq!(
            substitute("${env.REGISTRY}", &vars, Site::EnvValue, &at()).unwrap(),
            "registry.example"
        );
        assert!(substitute("${env.REGISTRY}", &vars, Site::Argv, &at()).is_err());
    }

    #[test]
    fn an_unset_env_read_is_bad_config_naming_the_variable() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        let err = substitute("${env.NOPE}", &vars, Site::EnvValue, &at()).unwrap_err();
        assert!(err.message.contains("NOPE"));
    }

    #[test]
    fn an_undeclared_port_is_bad_config_naming_the_port() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        let err = substitute("${port.nope}", &vars, Site::Argv, &at()).unwrap_err();
        assert!(err.message.contains("nope"));
    }

    /// `${files}` in a `commands:` entry is `bad_config` rather than a silently
    /// empty expansion — a command that quietly checks nothing is the failure
    /// the empty-set rule exists to prevent (PLAN.md §4.5).
    #[test]
    fn files_in_a_commands_entry_is_bad_config() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        assert!(expand_argv("script ${files}", &vars, &at()).is_err());
        assert!(substitute("${files}", &vars, Site::Argv, &at()).is_err());
    }

    /// The measured injection case: these filenames are legal on POSIX and git
    /// emits them raw. Splitting first means each arrives as exactly one
    /// argument, whatever it contains.
    #[test]
    fn a_hostile_filename_stays_one_argument() {
        let ports = ports();
        let env = env();
        let files = vec![
            "sub/semi;echo INJECTED.py".to_string(),
            "a file.py".to_string(),
        ];
        let vars = Vars {
            workspace_id: "a3f91c02",
            ports: &ports,
            component_root: None,
            files: Some(&files),
            env: &env,
        };
        assert_eq!(
            expand_argv("ruff check ${files}", &vars, &at()).unwrap(),
            vec!["ruff", "check", "sub/semi;echo INJECTED.py", "a file.py"]
        );
    }

    /// The same trust boundary, one layer in: a filename is *data*, so char's
    /// own placeholder syntax inside one is never re-scanned. The second name
    /// would otherwise fail `bad_config` and take the whole check with it.
    #[test]
    fn a_filename_is_inserted_verbatim_and_never_re_expanded() {
        let ports = ports();
        let env = env();
        let files = vec!["${port.api}.py".to_string(), "${nonsense}.py".to_string()];
        let vars = Vars {
            workspace_id: "a3f91c02",
            ports: &ports,
            component_root: None,
            files: Some(&files),
            env: &env,
        };
        assert_eq!(
            expand_argv("ruff check src/${files}", &vars, &at()).unwrap(),
            vec!["ruff", "check", "src/${port.api}.py", "src/${nonsense}.py"]
        );
    }

    #[test]
    fn an_empty_file_set_removes_the_token_rather_than_leaving_an_empty_argument() {
        let ports = ports();
        let env = env();
        let files: Vec<String> = Vec::new();
        let vars = Vars {
            workspace_id: "a3f91c02",
            ports: &ports,
            component_root: None,
            files: Some(&files),
            env: &env,
        };
        assert_eq!(
            expand_argv("ruff check ${files}", &vars, &at()).unwrap(),
            vec!["ruff", "check"]
        );
    }

    #[test]
    fn the_split_honours_quotes_and_backslashes() {
        assert_eq!(
            split_argv(r#"sh -c 'echo "a b"' plain\ word"#, &at()).unwrap(),
            vec!["sh", "-c", r#"echo "a b""#, "plain word"]
        );
    }

    #[test]
    fn the_split_expands_nothing_of_its_own() {
        assert_eq!(
            split_argv("go test ./... | tee log", &at()).unwrap(),
            vec!["go", "test", "./...", "|", "tee", "log"]
        );
    }

    #[test]
    fn an_unterminated_quote_is_bad_config() {
        assert!(split_argv("echo 'unclosed", &at()).is_err());
        assert!(split_argv("echo \"unclosed", &at()).is_err());
    }

    #[test]
    fn an_empty_command_is_bad_config() {
        assert!(split_argv("   ", &at()).is_err());
    }

    #[test]
    fn shell_quoting_survives_an_embedded_single_quote() {
        assert_eq!(shell_quote("plain"), "plain");
        assert_eq!(shell_quote("two words"), "'two words'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_quote(";rm -rf /"), "';rm -rf /'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn a_shell_command_is_wrapped_for_sh_dash_c() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        assert_eq!(
            shell_argv("a && b", &vars, &at()).unwrap(),
            vec!["/bin/sh", "-c", "a && b"]
        );
    }

    #[test]
    fn an_unterminated_placeholder_is_left_alone() {
        let ports = ports();
        let env = env();
        let vars = Vars::new("a3f91c02", &ports, &env);
        assert_eq!(
            substitute("cost is ${100", &vars, Site::Argv, &at()).unwrap(),
            "cost is ${100"
        );
    }
}
