//! **Where the Armada-written half of a guild came from**, so that a later
//! release can reach it ([`006`](../../../docs/reserved/006-guild-has-no-way-to-learn.md)).
//!
//! `templates/guild/subagents/helm.md` says *"it is yours from that moment, and
//! nothing here is updated by a later Armada release."* That sentence is right
//! for `voice.md` and wrong for the persona: how to delegate and what to verify
//! is operating knowledge Armada learns, and today no existing guild can ever
//! receive it.
//!
//! # The blocker was provenance, not merging
//!
//! Nothing recorded which template set a guild's files came from, so there was
//! no **base** — and without a base there is no three-way merge, only overwrite
//! or keep, and both are wrong. This module is the base: [`STAMP`] is the file
//! that records it and [`digest`] is what it records.
//!
//! # No merge engine gets written
//!
//! The guild is already a git repository ([`crate::repo`]), so Armada's
//! templates ship as a branch inside it — [`BRANCH`]. The user's edits stay on
//! `main`, each release appends one commit to `armada`, and `armada guild
//! upgrade` is `git merge`. Git tracks the base, merges an untouched file
//! silently, and conflicts only where both sides changed the same lines. The
//! repository already contains the merge engine; borrowing it is the whole
//! design.
//!
//! # Not everything takes updates
//!
//! | File | Takes updates? | Why |
//! |---|---|---|
//! | [`STAMP`] | Always | it is the record of what an upgrade did |
//! | `workflows/workflow.schema.json` | Always | it is Armada's — workflows are checked against it |
//! | `subagents/helm.md` | Yes | operating knowledge, not personal |
//! | `skills/onboard-repo/SKILL.md` | Offer | it may have been customised |
//! | `voice.md`, `how-i-work.md`, `expectations.md` | **Never** | these are you |
//!
//! **"Never" is enforced by absence, not by a check.** The three fragments are
//! not in [`MANAGED`], so they are never written onto the `armada` branch, so
//! the merge has nothing to say about them. A rule that depends on a conditional
//! being right every time is a rule that is wrong once; this one cannot fire.
//! [`no_managed_file_is_one_of_yours`](tests::no_managed_file_is_one_of_yours)
//! holds the two lists apart.
//!
//! **The four starter workflows and `permissions.yml` are also absent**, and
//! deliberately: the interview writes your ceilings into them and your posture
//! into it, so an update carrying Armada's numbers back over yours is the same
//! failure as overwriting `voice.md`, one level less obvious. `006` names the
//! gap that leaves — a schema can gain a predicate no starter workflow uses —
//! and it is recorded there rather than closed here by guessing.

use serde::Deserialize;

/// The branch Armada's templates live on, inside the guild's own repository.
///
/// **Named after the tool rather than after `upstream`**, because a person who
/// opens `~/.armada/guild` in a git client should be able to tell at a glance
/// which branch is theirs and which one is not.
pub const BRANCH: &str = "armada";

/// The file that records which template set the guild's managed files came
/// from.
///
/// At the guild's root, in the guild's own vocabulary, and **readable** —
/// `armada guild ls` prints what is in it, because a version nobody can see is
/// one nobody trusts.
pub const STAMP: &str = "templates.yml";

/// Which Armada wrote the templates.
///
/// **Recorded alongside [`digest`] rather than instead of it.** `AGENTS.md`
/// says the package version carries no compatibility signal, and it does not:
/// what identifies a template set is its content. The version is here for the
/// one decision content cannot make — whether the guild has already been
/// upgraded by a *newer* Armada than the one now running, which is a downgrade
/// and is refused ([`newer`]).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whether a managed file takes updates, and on what terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// It is Armada's. It updates, and there is nothing to decide.
    Always,
    /// It is operating knowledge rather than yours. It updates.
    Yes,
    /// It may have been customised, so it is offered and not taken.
    Offer,
}

impl Policy {
    /// Whether this file is written onto the upstream branch on a given run.
    ///
    /// One argument, and it is the only thing that varies: whether the caller
    /// accepted the offer.
    pub const fn takes(self, offered: bool) -> bool {
        match self {
            Policy::Always | Policy::Yes => true,
            Policy::Offer => offered,
        }
    }

    /// The word an upgrade's DETAIL column uses when it declined this file.
    pub const fn word(self) -> &'static str {
        match self {
            Policy::Always => "Armada's",
            Policy::Yes => "operating knowledge",
            Policy::Offer => "yours to take",
        }
    }
}

/// One file Armada may update, and the terms it updates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Managed {
    /// Guild-relative.
    pub path: &'static str,
    /// Whether it takes updates.
    pub policy: Policy,
}

/// **Every file an upgrade may touch, and there is no other list.**
///
/// The set is small on purpose. Anything absent from here cannot be written by
/// an upgrade even by a bug, because the upgrade's only mechanism is writing
/// these paths onto [`BRANCH`] — a path that never appears there is a path the
/// merge has nothing to say about.
pub const MANAGED: [Managed; 4] = [
    Managed {
        path: STAMP,
        policy: Policy::Always,
    },
    Managed {
        path: "workflows/workflow.schema.json",
        policy: Policy::Always,
    },
    Managed {
        path: "subagents/helm.md",
        policy: Policy::Yes,
    },
    Managed {
        path: "skills/onboard-repo/SKILL.md",
        policy: Policy::Offer,
    },
];

/// The managed files this run writes, given whether the offer was accepted.
pub fn taking(offered: bool) -> Vec<Managed> {
    MANAGED
        .into_iter()
        .filter(|managed| managed.policy.takes(offered))
        .collect()
}

/// The managed files this run declined — offered, and not accepted.
pub fn offering(offered: bool) -> Vec<Managed> {
    MANAGED
        .into_iter()
        .filter(|managed| !managed.policy.takes(offered))
        .collect()
}

/// What identifies a template set: **its content, hashed**.
///
/// Over the managed paths and their bodies, in [`MANAGED`]'s order, with a
/// separator between every field so that moving a byte from a path into a body
/// cannot produce the same digest. [`STAMP`] is excluded because the stamp
/// records this value, and a hash over a file containing itself has no answer.
///
/// The workspace's one hash, for the reason the pin exists: two hashes is two
/// readings of one guild that stop agreeing across an upgrade, which here would
/// mean an upgrade that believes it has nothing to do.
pub fn digest() -> String {
    let mut hasher = blake3::Hasher::new();
    for managed in MANAGED {
        // The stamp *records* this value, and a hash over a file containing
        // itself has no answer. Skipped here rather than in `body_of`, so that
        // the exclusion is where the recursion would be.
        if managed.path == STAMP {
            continue;
        }
        let Some(body) = crate::starters::starter(managed.path).map(|s| s.body) else {
            continue;
        };
        hasher.update(managed.path.as_bytes());
        hasher.update(b"\0");
        hasher.update(body.as_bytes());
        hasher.update(b"\0");
    }
    hasher.finalize().to_hex()[..12].to_string()
}

/// The shipped body of a managed file, or `None` for [`STAMP`], which is
/// generated rather than shipped.
pub fn body_of(path: &str) -> Option<String> {
    if path == STAMP {
        return Some(stamp());
    }
    crate::starters::starter(path).map(|starter| starter.body.to_string())
}

/// The stamp file, written out.
///
/// **Prose in it, like every other file Armada writes into a guild.** A reader
/// who opens this wants to know whether it is theirs to edit, and the answer is
/// interesting: editing it changes what Armada believes the base is, which is
/// the one thing an upgrade cannot work out for itself.
pub fn stamp() -> String {
    format!(
        "# Which Armada templates the Armada-written part of this guild came from.\n\
         #\n\
         # Written by `armada guild init`, advanced by `armada guild upgrade`, and\n\
         # printed by `armada guild ls`. This is not your content: it is the base a\n\
         # three-way merge needs, and editing it only changes what Armada believes\n\
         # your files came from.\n\
         #\n\
         # Your voice, expectations and how-i-work are never updated by a release.\n\
         version: {VERSION}\n\
         templates: {}\n",
        digest()
    )
}

/// What a guild's [`STAMP`] says, when it has one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Stamp {
    /// The Armada that last wrote the managed files.
    pub version: String,
    /// [`digest`], as it was then.
    pub templates: String,
}

impl Stamp {
    /// `0.1.0 8f3a1c0d9e21` — how `armada guild ls` prints it.
    pub fn written(&self) -> String {
        format!("{} {}", self.version, self.templates)
    }
}

/// Read a stamp, or `None` for a guild that has none — which is every guild
/// created before this shipped, and is the case that has to work.
pub fn read(text: &str) -> Option<Stamp> {
    serde_yaml_ng::from_str(text).ok()
}

/// Whether `theirs` is a later Armada than `ours`.
///
/// **The one comparison a digest cannot make.** Two digests are equal or they
/// are not; nothing in them says which came first. So the guard against
/// upgrading a guild *backwards* — a second machine running an older binary,
/// which would take Armada's older text over its newer text and conflict on
/// every line both releases touched — is read off the version instead.
///
/// Numeric field by field, with a non-numeric field read as zero: `0.10.0` is
/// later than `0.9.0`, which a string comparison gets wrong.
pub fn newer(theirs: &str, ours: &str) -> bool {
    fields(theirs) > fields(ours)
}

fn fields(version: &str) -> Vec<u64> {
    version
        .split(['.', '-', '+'])
        .map(|field| field.parse().unwrap_or(0))
        .collect()
}

/// The commits `armada guild init` writes, and **the only marks a
/// pre-provenance guild carries**.
///
/// A guild created before [`STAMP`] existed has no record of where its files
/// came from — except these. `init` commits under one of these two subjects and
/// nothing else does, so the newest of them is the last moment Armada wrote the
/// managed files, which is exactly the base a first upgrade needs.
pub const WRITTEN_BY_INIT: [&str; 2] = [
    "guild: imported and interviewed",
    "guild: re-initialised",
];

/// The commit a guild with no [`STAMP`] adopts as its base.
///
/// Given `git log --format=%H %s`, newest first: the first line whose subject
/// is one of [`WRITTEN_BY_INIT`], falling back to the oldest commit in the log.
///
/// **The fallback is the root commit and it is a guess, stated as one.** A
/// guild whose history was rewritten, or which was assembled by hand, has no
/// moment Armada can point at — so an upgrade of one reports which commit it
/// adopted, because a base nobody was told about is a base nobody can dispute.
pub fn base_of(log: &str) -> Option<String> {
    let lines: Vec<&str> = log.lines().filter(|line| !line.trim().is_empty()).collect();
    lines
        .iter()
        .find(|line| {
            line.split_once(' ')
                .is_some_and(|(_, subject)| WRITTEN_BY_INIT.contains(&subject.trim()))
        })
        .or_else(|| lines.last())
        .and_then(|line| line.split_whitespace().next())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The rule the module exists to make unbreakable.** The three fragments
    /// are you, and they are kept out of reach by not being in the list at all
    /// rather than by a check that has to fire correctly every time.
    #[test]
    fn no_managed_file_is_one_of_yours() {
        for fragment in crate::memory::FRAGMENTS {
            assert!(
                !MANAGED.iter().any(|managed| managed.path == fragment),
                "`{fragment}` is you, and an upgrade would overwrite it"
            );
        }
        // The starter workflows and the posture carry the interview's answers,
        // so they are yours in the same way for a less obvious reason.
        for yours in [
            "workflows/bug.yml",
            "workflows/plan.yml",
            "workflows/design.yml",
            "workflows/feature.yml",
            crate::permissions::FILE,
        ] {
            assert!(
                !MANAGED.iter().any(|managed| managed.path == yours),
                "`{yours}` holds your interview answers"
            );
        }
    }

    /// Every managed file is one Armada actually ships, so an upgrade cannot
    /// write a path with nothing behind it.
    #[test]
    fn every_managed_file_is_one_armada_ships() {
        for managed in MANAGED {
            let body = body_of(managed.path)
                .unwrap_or_else(|| panic!("`{}` is managed and not shipped", managed.path));
            assert!(body.len() > 100, "`{}` is nearly empty", managed.path);
        }
    }

    /// The persona is the file `006` was raised about — the six verification
    /// lessons and the model-tier policy that no existing guild can receive.
    #[test]
    fn the_persona_takes_updates_and_the_skill_is_offered() {
        let persona = MANAGED
            .iter()
            .find(|m| m.path == "subagents/helm.md")
            .expect("the persona is managed");
        assert_eq!(persona.policy, Policy::Yes);
        assert!(persona.policy.takes(false), "it updates without being asked");

        let skill = MANAGED
            .iter()
            .find(|m| m.path == "skills/onboard-repo/SKILL.md")
            .expect("the skill is managed");
        assert_eq!(skill.policy, Policy::Offer);
        assert!(!skill.policy.takes(false), "an offer is not a default");
        assert!(skill.policy.takes(true));
    }

    /// Declining the offer leaves the other three, and taking it leaves none
    /// behind. The two functions are complements and a file in neither would be
    /// one no row ever reports.
    #[test]
    fn taking_and_offering_partition_the_managed_set() {
        for offered in [false, true] {
            let taken = taking(offered).len();
            let left = offering(offered).len();
            assert_eq!(taken + left, MANAGED.len(), "a managed file went missing");
        }
        assert_eq!(taking(false).len(), 3);
        assert_eq!(offering(false).len(), 1);
        assert_eq!(offering(true).len(), 0);
    }

    /// The digest is over content, so it is stable within a build and short
    /// enough to read off a summary line.
    #[test]
    fn the_digest_is_content_addressed_and_short() {
        assert_eq!(digest().len(), 12);
        assert_eq!(digest(), digest());
        assert!(digest().chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// The stamp is a file Armada writes and Armada reads, so the two have to
    /// agree — and the round trip is the only thing that proves they do.
    #[test]
    fn the_stamp_reads_back_as_what_was_written() {
        let stamped = read(&stamp()).expect("the stamp Armada writes parses");
        assert_eq!(stamped.version, VERSION);
        assert_eq!(stamped.templates, digest());
        assert_eq!(stamped.written(), format!("{VERSION} {}", digest()));
        // The prose survives, because a reader who opens it needs to know
        // whether it is theirs to edit.
        assert!(stamp().starts_with('#'));
        assert!(stamp().contains("armada guild upgrade"));
    }

    /// A guild with no stamp is the case that has to work, and it reads as an
    /// absence rather than as a failure.
    #[test]
    fn a_guild_with_no_stamp_reads_as_none() {
        assert!(read("").is_none());
        assert!(read("nothing: here\n").is_none());
    }

    /// **`0.10.0` is later than `0.9.0`**, which is the comparison a string
    /// gets wrong and the reason this is not one.
    #[test]
    fn a_later_armada_is_recognised_field_by_field() {
        assert!(newer("0.10.0", "0.9.0"));
        assert!(!newer("0.9.0", "0.10.0"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(newer("0.1.1", "0.1.0"));
        assert!(newer("1.0.0", "0.99.99"));
        // A version nobody can parse is not "later", because refusing an
        // upgrade on an unreadable field is refusing it forever.
        assert!(!newer("nonsense", "0.1.0"));
    }

    /// **The pre-provenance base**, which is the only case that matters here:
    /// his guild exists, it has no stamp, and the last time Armada wrote its
    /// managed files is the newest `guild: re-initialised`.
    #[test]
    fn the_base_is_the_last_commit_armada_wrote_the_templates_in() {
        let log = "99894d9 guild: re-initialised\n\
                   03c2eeb guild: re-initialised\n\
                   c401123 guild: imported and interviewed\n";
        assert_eq!(base_of(log).unwrap(), "99894d9");
    }

    /// A guild edited since its last init still bases on the init, not on the
    /// edit — the edit is the user's and is exactly what the merge must keep.
    #[test]
    fn edits_after_an_init_do_not_move_the_base() {
        let log = "aaa1111 guild: edit voice.md\n\
                   bbb2222 guild: imported and interviewed\n";
        assert_eq!(base_of(log).unwrap(), "bbb2222");
    }

    /// **A history with no init commit falls back to the oldest**, and the
    /// verb says which commit it adopted. A base nobody was told about is a
    /// base nobody can dispute.
    #[test]
    fn a_history_armada_did_not_write_falls_back_to_the_oldest_commit() {
        let log = "aaa1111 assorted\nbbb2222 by hand\n";
        assert_eq!(base_of(log).unwrap(), "bbb2222");
        assert!(base_of("").is_none());
    }
}
