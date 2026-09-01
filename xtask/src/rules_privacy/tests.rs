//! The rule's negative tests, and the false positives it must not raise.
//!
//! Every name here is invented. Writing the owner's real name into a test that
//! keeps his name out of files would defeat itself — `#269` settled that — and
//! it would also make the environment half untestable, because it would pass
//! for one person and fail for everyone else. The name the gate compares
//! against is passed in instead, so these run the same on any machine.

use super::*;

/// What `leaks_in` found, as sentences, for readable assertions.
fn said(line: &str, running_as: &[&str]) -> Vec<String> {
    let names: Vec<String> = running_as.iter().map(|n| n.to_string()).collect();
    leaks_in(line, &names)
        .iter()
        .map(|leak| leak.sentence())
        .collect()
}

// ------------------------------------------------- part one: every occurrence

/// The bug this fixes. `split(prefix).nth(1)` reads the segment after the
/// *first* `/Users/` and stops, so this line reported one leak and there are
/// three. Thirteen lines stood for a hundred and twenty-three occurrences.
#[test]
fn three_home_directories_on_one_line_are_three_findings() {
    let line = "cp /Users/ada/a /Users/ada/b && mv /Users/ada/c /tmp";
    assert_eq!(said(line, &[]).len(), 3);
}

/// Two different people on one line, under both prefixes. The old code could
/// see at most one of each, and only the first.
#[test]
fn both_prefixes_and_both_people_are_named() {
    let line = "/Users/ada/x /Users/grace/y /home/turing/z /home/hopper/w";
    let found = said(line, &[]);
    assert_eq!(found.len(), 4, "{found:?}");
    assert!(found.iter().any(|s| s.contains("`grace`")), "{found:?}");
    assert!(found.iter().any(|s| s.contains("`hopper`")), "{found:?}");
}

/// The convention itself is not a leak, however many times it appears.
#[test]
fn the_convention_name_is_never_a_leak() {
    assert!(said("/Users/user/a /Users/user/b /home/user/c", &[]).is_empty());
}

// ---------------------------------------------- part two: the name it runs as

/// The twenty occurrences the rule could not see: a username as an `lsof`
/// owner column, with no path anywhere near it.
#[test]
fn a_bare_owner_column_is_refused() {
    let line = "curl    6340 ada    5u  IPv4 0x2501c007d5eee926      0t0  TCP";
    let found = said(line, &["ada"]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("running as"), "{found:?}");
}

/// A name is a word, not a substring. `ada` inside `adaptive` is not a person,
/// and a gate that says it is fires on ordinary prose forever.
#[test]
fn a_name_inside_a_longer_word_is_not_a_person() {
    assert!(said("the adaptive scheduler and its metadata", &["ada"]).is_empty());
}

/// The other three shapes `78da9e93` redacted, and the one that would have
/// been missed if a hyphen held a word together: a project directory with its
/// path separators mangled.
#[test]
fn every_captured_shape_of_the_bare_name_is_refused() {
    assert_eq!(said("HOME=/Users/ada", &["ada"]).len(), 2, "home and name");
    assert_eq!(said("USER=ada", &["ada"]).len(), 1);
    assert_eq!(
        said("/private/tmp/-Users-ada-Development-armada", &["ada"]).len(),
        1
    );
}

/// **The trade this half makes.** It matches byte for byte, so the same name in
/// another case goes unremarked. That is deliberate: a capture reproduces the
/// environment exactly, and prose does not — the repository's own GitHub slug
/// carries its owner's account name in a different case, and a case-insensitive
/// match reddens the clone line in `README.md`, which cannot be written without
/// it. This was measured against the tree, not guessed.
#[test]
fn the_same_name_in_another_case_is_a_different_string() {
    assert!(said("https://github.com/Ada/armada.git", &["ada"]).is_empty());
    assert_eq!(said("https://github.com/ada/armada.git", &["ada"]).len(), 1);
}

/// Git's `user.name` has a space in it. It is matched as written.
#[test]
fn a_two_word_name_matches_as_one_subject() {
    let found = said("Ada Lovelace <ada@example.invalid>", &["Ada Lovelace"]);
    assert!(
        found.iter().any(|s| s.contains("`Ada Lovelace`")),
        "{found:?}"
    );
}

/// **The weakness, made a test.** On a machine belonging to someone else the
/// gate knows no name, and this half finds nothing. The paths convention is
/// what still carries the load there.
#[test]
fn a_gate_that_knows_no_name_finds_no_name() {
    let line = "curl    6340 ada    5u  IPv4 0x2501c007d5eee926      0t0  TCP";
    assert!(said(line, &[]).is_empty());
}

/// The degenerate cases that would turn the gate into noise: unset reaches
/// here as empty, whitespace is empty, the convention matches every committed
/// capture, and one or two characters is a word in ordinary prose far more
/// often than it is a person.
#[test]
fn a_name_that_would_be_noise_is_not_a_name() {
    assert_eq!(usable_name(""), None);
    assert_eq!(usable_name("   \n"), None);
    assert_eq!(usable_name("user"), None);
    assert_eq!(usable_name("User"), None);
    assert_eq!(usable_name("ci"), None);
    assert_eq!(usable_name(" ada \n").as_deref(), Some("ada"));
}

// ----------------------------------------------- part three: a machine's name

/// `v1-final:docs/traps.md:1410` in the shape it was captured — a person and a
/// machine on one line, in the tag `CLAUDE.md` sends readers to.
#[test]
fn a_bonjour_hostname_is_a_machine() {
    let found = said("Ada Lovelace <ada@Adas-MacBook-Pro.local>", &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`Adas-MacBook-Pro.local`"), "{found:?}");
}

/// A hostname on the other suffix, and lowercase — an `@` or a `/` in front of
/// it says it is a host and not a field.
#[test]
fn a_lowercase_host_behind_an_at_or_a_slash_is_a_machine() {
    assert_eq!(said("ssh ada@imac.lan", &[]).len(), 1);
    assert_eq!(said("curl http://imac.local:8953/health", &[]).len(), 1);
}

/// `localhost` is every machine and therefore no machine.
#[test]
fn localhost_is_not_a_machine() {
    assert!(said("127.0.0.1 localhost", &[]).is_empty());
    assert!(said("http://localhost.local/", &[]).is_empty());
    assert!(said("bound to localhost:8953", &[]).is_empty());
}

/// The false positive that is checked in seven times today. `local_addr` is a
/// method, and a rule that reddens `crates/fleet/src/tests/peer.rs` is a rule
/// somebody turns off.
#[test]
fn a_local_addr_call_is_not_a_machine() {
    assert!(said("let bound = listener.local_addr()?;", &[]).is_empty());
    assert!(said("stream.local_addr().expect(\"its port\").port()", &[]).is_empty());
}

/// The other `.local` that is checked in: a dotfile directory, where the host
/// would have to be empty.
#[test]
fn a_dot_local_directory_is_not_a_machine() {
    assert!(said("~/.local/bin/claude", &[]).is_empty());
    assert!(said("\"ps\": \"5752  5738 ~/.local/bin/claude\"", &[]).is_empty());
}

/// A filename with `.local` in the middle of it is a filename.
#[test]
fn a_local_filename_is_not_a_machine() {
    assert!(said("cp .env.local .env", &[]).is_empty());
    assert!(said("vite.config.local.ts", &[]).is_empty());
    assert!(said("resolves against localdomain", &[]).is_empty());
}

// ----------------------------------------------------------- what a file says

/// **The noise this rule must not make.** Twenty lines carrying one fact are
/// one finding with the honest count, not twenty findings. Sixty-one comment
/// lines hid thirteen privacy lines all night for exactly this reason.
#[test]
fn a_file_naming_one_person_twenty_times_says_so_once() {
    let text = "open /Users/ada/notes\n".repeat(20);
    let found = findings_in("docs/spikes/capture.jsonl", &text, &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("20 times in this file"), "{found:?}");
    assert!(
        found[0].starts_with("docs/spikes/capture.jsonl:1 —"),
        "{found:?}"
    );
}

/// Collapsing is per subject, not per file. Two people are two findings.
#[test]
fn two_people_in_one_file_are_two_findings() {
    let text = "/Users/ada/x\n/Users/grace/y\n/Users/ada/z\n";
    let found = findings_in("docs/a.md", text, &[]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().any(|s| s.contains("2 times")), "{found:?}");
}

/// The one file that names a person on purpose. Apache-2.0 requires the
/// copyright holder by name, and a licence with the name taken out is not a
/// licence. Everywhere else the same line is a leak.
#[test]
fn the_licence_may_name_its_copyright_holder_and_nothing_else_may() {
    let text = "   Copyright 2026 Ada Lovelace\n";
    let names = vec!["Ada Lovelace".to_string()];
    assert!(findings_in("LICENSE", text, &names).is_empty());
    assert_eq!(findings_in("README.md", text, &names).len(), 1);
}

/// A home path in the licence is still a home path. The exemption is for the
/// copyright holder's name, not for the file.
#[test]
fn the_licence_is_not_exempt_from_the_paths_convention() {
    let found = findings_in("LICENSE", "see /Users/ada/notes\n", &[]);
    assert_eq!(found.len(), 1, "{found:?}");
}

/// The credential half is untouched: it reports the line it found, every time
/// it finds one, and does not collapse.
#[test]
fn a_credential_is_still_reported_on_every_line() {
    let text = "token: ghp_aaaa\nnothing\ntoken: ghp_bbbb\n";
    let found = findings_in("docs/a.md", text, &[]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with("docs/a.md:1 —"), "{found:?}");
    assert!(found[1].starts_with("docs/a.md:3 —"), "{found:?}");
}
