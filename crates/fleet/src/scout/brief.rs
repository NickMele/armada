//! What a scout is told. `#1292`.
//!
//! The wording is `docs/contracts/agent-prompt.md` section 5b, drafted, and
//! transcribed here. A change to it belongs in the contract first.

const OPENING: &str = "\
You are a scout, in Armada. A person working out what to do next asked a \
question about the code in one repository, and you answer it by reading that \
repository's checkout.";

const THE_REPOSITORY: &str = "\
THE REPOSITORY

Its checkout is your working directory, {root}. You can read, search and list \
files inside it, and nothing outside it. You read it as it is on disk, \
uncommitted changes included.";

const MAY_DO: &str = "\
WHAT YOU MAY DO

You read and never write. You have no tool that edits a file, runs a command, \
commits or reaches the network, and you do not ask for one. Where an answer \
needs something you cannot read, say what it is and stop there.";

const ANSWER: &str = "\
WHAT YOU ANSWER WITH

Armada lists every file you read and every search you run beside your answer, \
so you do not list them. End with the answer itself: what the code does, \
naming the files it rests on. Where you are inferring rather than reading, say \
so.";

/// A scout's one turn, whole: the brief, then the ask verbatim.
pub(crate) fn told(root: &str, asked: &str) -> String {
    [
        OPENING,
        &THE_REPOSITORY.replace("{root}", root),
        MAY_DO,
        ANSWER,
        &format!("THE ASK\n\n{asked}"),
    ]
    .join("\n\n")
}

#[cfg(test)]
mod tests {
    /// **The contract's drafted wording, whole**: section 5b, with its two
    /// slots filled.
    #[test]
    fn a_scout_is_told_the_contracts_brief_with_the_ask_last_and_verbatim() {
        let told = super::told("/repos/armada", "how is routing decided?\nAnd why?");
        let contract = include_str!("../../../../docs/contracts/agent-prompt.md");
        let section = contract
            .split("# 5b. The scout brief")
            .nth(1)
            .expect("section 5b");
        let drafted = section
            .split("```\n")
            .nth(1)
            .expect("the drafted block")
            .trim_end()
            .replace("{root}", "/repos/armada")
            .replace("{asked}", "how is routing decided?\nAnd why?");
        let unwrapped = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(unwrapped(&told), unwrapped(&drafted));
        assert!(told.ends_with("THE ASK\n\nhow is routing decided?\nAnd why?"));
    }
}
