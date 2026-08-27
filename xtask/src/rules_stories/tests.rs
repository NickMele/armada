//! The transform's own tests, and the rule's negative ones.
//!
//! The awkward titles are the point. A transform that only ever meets
//! `Job brief` is a transform nobody has tested — every case that could have
//! forced it to widen is checked in already: an em dash, parentheses, a comma,
//! a lowercase name, an interior capital and a digit glued to a letter.

use super::*;
use crate::Finding;

/// Every title checked in on the day the rule was written, with the directory
/// that holds it. The rule is proved against these rather than against a walk
/// of the tree, which can only ever be in one state at a time.
const CHECKED_IN: &[(&str, &str)] = &[
    ("Job brief", "JobBrief"),
    ("Job row (stacked)", "JobRowStacked"),
    ("kbd", "Kbd"),
    ("StatusBar", "StatusBar"),
    ("CommandPalette", "CommandPalette"),
    ("Tabs with counts", "TabsWithCounts"),
    ("Split button", "SplitButton"),
    ("A running job", "ARunningJob"),
    (
        "A failed job — a dead end, read as one",
        "AFailedJobADeadEndReadAsOne",
    ),
    (
        "A finished job — a branch and an evidence trail",
        "AFinishedJobABranchAndAnEvidenceTrail",
    ),
    (
        "The list — six states, one row shape",
        "TheListSixStatesOneRowShape",
    ),
    (
        "Dispatch a job — full, with the M1 subset marked",
        "DispatchAJobFullWithTheM1SubsetMarked",
    ),
];

#[test]
fn the_transform_survives_every_title_in_the_tree() {
    for (title, dir) in CHECKED_IN {
        assert_eq!(&pascal(title), dir, "{title}");
    }
}

/// An em dash is a separator and not a letter, so the words either side do not
/// fuse. This is the case that would silently produce `AFailedJob—ADeadEnd`
/// under a transform that only stripped ASCII punctuation.
#[test]
fn an_em_dash_separates_rather_than_survives() {
    assert_eq!(pascal("A failed job — a dead end"), "AFailedJobADeadEnd");
}

/// `kbd` and `StatusBar` pull in opposite directions: one needs its first
/// letter raised, the other needs its interior capital left alone. A transform
/// that lowercases the tail of a word satisfies the first and breaks the
/// second.
#[test]
fn only_the_first_letter_of_a_word_changes() {
    assert_eq!(pascal("kbd"), "Kbd");
    assert_eq!(pascal("StatusBar"), "StatusBar");
    assert_eq!(pascal("Tabs with counts"), "TabsWithCounts");
}

/// A digit is alphanumeric, so `M1` stays one word rather than becoming `M`
/// and a dropped `1`.
#[test]
fn a_digit_stays_inside_its_word() {
    assert_eq!(pascal("the M1 subset"), "TheM1Subset");
}

/// Runs of punctuation, and punctuation at either end, produce no empty words.
#[test]
fn punctuation_never_produces_an_empty_word() {
    assert_eq!(pascal("(stacked)"), "Stacked");
    assert_eq!(pascal("a — b, c"), "ABC");
    assert_eq!(pascal("   "), "");
}

/// A non-ASCII letter is dropped rather than carried, which makes the title
/// fail against any directory instead of quietly matching a widened one.
#[test]
fn a_non_ascii_letter_breaks_the_word_rather_than_widening_the_rule() {
    assert_eq!(pascal("café order"), "CafOrder");
}

/// A story file in the shape every one in the tree has.
fn story(title: &str) -> String {
    format!(
        "import type {{ Meta }} from \"@storybook/react-vite\";\n\
         \n\
         const meta: Meta<typeof Thing> = {{\n  \
         title: \"{title}\",\n  \
         component: Thing,\n\
         }};\n\
         export default meta;\n"
    )
}

#[test]
fn the_meta_title_is_read_with_its_line() {
    let (title, line) = meta_title(&story("Compositions/Job brief")).expect("a title");
    assert_eq!(title, "Compositions/Job brief");
    assert_eq!(line, 4);
}

/// `TheShell` carries a fixture whose `title` sits at the same indentation as
/// the meta's. Taking the first `title:` in the file reads `Active jobs` as the
/// story's title and reports a mismatch against a correct directory.
#[test]
fn a_fixtures_title_is_not_mistaken_for_the_metas() {
    let text = "const meta: Meta<typeof TheShell> = {\n  \
                title: \"Screens/The shell\",\n\
                };\n\
                const shell = {\n  \
                title: \"Active jobs\",\n\
                };\n";
    assert_eq!(meta_title(text).expect("a title").0, "Screens/The shell");
}

/// A `title` above the meta belongs to something else, and a meta that closes
/// without one has none.
#[test]
fn a_title_outside_the_meta_block_is_not_the_metas() {
    let text = "const preview = {\n  title: \"Not it\",\n};\n\
                const meta: Meta = {\n  component: Thing,\n};\n";
    assert!(meta_title(text).is_none());
}

/// A tree laid out under a temporary root, so each direction of the rule is
/// proved against a mismatch built here.
struct Tree(std::path::PathBuf);

impl Tree {
    fn new(name: &str) -> Tree {
        let dir = std::env::temp_dir().join(format!("armada-stories-{name}"));
        let _ = fs::remove_dir_all(&dir);
        Tree(dir)
    }

    /// One component directory. `None` for either file leaves it out.
    fn component(self, group: &str, name: &str, title: Option<&str>, code: bool) -> Tree {
        let dir = self.0.join(ROOT).join(group).join(name);
        fs::create_dir_all(&dir).expect("a component directory");
        if let Some(title) = title {
            fs::write(dir.join(format!("{name}.stories.tsx")), story(title)).expect("a story");
        }
        if code {
            fs::write(
                dir.join(format!("{name}.tsx")),
                "export const Thing = () => null;\n",
            )
            .expect("a component");
        }
        self
    }

    fn loose_story(self, at: &str, title: &str) -> Tree {
        let path = self.0.join(ROOT).join(at);
        fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        fs::write(&path, story(title)).expect("a story");
        self
    }

    /// Every failing line the rule produces against this tree.
    fn run(self) -> Vec<String> {
        let report = every_story_names_its_own_path(&self.0);
        let _ = fs::remove_dir_all(&self.0);
        report
            .findings
            .iter()
            .map(|f| match f {
                Finding::Fail(what) | Finding::Warn(what) => what.clone(),
            })
            .collect()
    }
}

#[test]
fn a_matching_component_passes() {
    let lines = Tree::new("match")
        .component(
            "compositions",
            "JobRowStacked",
            Some("Compositions/Job row (stacked)"),
            true,
        )
        .run();
    assert!(lines.is_empty(), "{lines:?}");
}

#[test]
fn a_title_that_disagrees_with_its_directory_names_both() {
    let lines = Tree::new("mismatch")
        .component(
            "compositions",
            "JobBreif",
            Some("Compositions/Job brief"),
            true,
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("Compositions/Job brief"), "{lines:?}");
    assert!(lines[0].contains("`JobBrief`"), "{lines:?}");
    assert!(lines[0].contains("`JobBreif/`"), "{lines:?}");
}

#[test]
fn a_title_in_the_wrong_group_names_both() {
    let lines = Tree::new("group")
        .component(
            "primitives",
            "JobBrief",
            Some("Compositions/Job brief"),
            true,
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("group `Compositions`"), "{lines:?}");
    assert!(lines[0].contains("`primitives/`"), "{lines:?}");
}

#[test]
fn a_component_with_no_story_fails() {
    let lines = Tree::new("no-story")
        .component("primitives", "Button", None, true)
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("no `Button.stories.tsx`"), "{lines:?}");
}

#[test]
fn a_story_with_no_component_fails() {
    let lines = Tree::new("no-component")
        .component(
            "screens",
            "FirstLaunch",
            Some("Screens/First launch"),
            false,
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("no `FirstLaunch.tsx`"), "{lines:?}");
}

/// A story loose in a group directory is loaded by Storybook's glob and has no
/// directory to be checked against — the one way a story can exist that the
/// per-directory walk cannot see.
#[test]
fn a_story_outside_a_component_directory_fails() {
    let lines = Tree::new("loose")
        .loose_story("screens/Stray.stories.tsx", "Screens/Stray")
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("outside `<group>/Stray/Stray.stories.tsx`"),
        "{lines:?}"
    );
}

#[test]
fn a_title_with_no_group_fails() {
    let lines = Tree::new("no-group")
        .component("primitives", "Button", Some("Button"), true)
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("names no group"), "{lines:?}");
}

#[test]
fn a_title_nested_below_its_group_fails() {
    let lines = Tree::new("nested")
        .component(
            "primitives",
            "Button",
            Some("Primitives/Forms/Button"),
            true,
        )
        .run();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("nests below its group"), "{lines:?}");
}

#[test]
fn a_missing_library_names_itself() {
    let report = every_story_names_its_own_path(Path::new("/nonexistent/armada"));
    let lines: Vec<_> = report
        .findings
        .iter()
        .map(|f| match f {
            Finding::Fail(what) | Finding::Warn(what) => what.clone(),
        })
        .collect();
    assert_eq!(
        lines,
        vec![format!(
            "{ROOT} — the component library the stories live in"
        )]
    );
}
