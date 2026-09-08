import type { Meta, StoryObj } from "@storybook/react-vite";
import { Refusals } from "./Refusals";

const meta: Meta<typeof Refusals> = {
  title: "Compositions/Refusals",
  component: Refusals,
};
export default meta;

type Story = StoryObj<typeof Refusals>;

/** The sentence the band says over the rows, at every state that has any. */
const SAID = "The job was blocked from the following:";

/**
 * What a restart meets, said under every list `screens` builds. The words are
 * `refused.ts`'s; they are spelled here so the stories draw the shape the app
 * draws rather than a placeholder that reads shorter than the real one.
 */
const AGAIN =
  "A drone's toolset is fixed when it starts, and a restart builds the same one from the same " +
  "declaration. A drone that reaches for these again is refused again.";

/** The second sentence, on a list with a refused command in it. */
const DECLARED =
  "A command is in that toolset only where the repository's armada.yml declares it under " +
  "commands and does not mark it destructive.";

/**
 * **The report, as one row.** A job stopped saying it was blocked by policy,
 * and the only thing on screen was the policy — so the answer to "unblock it
 * from what" was to open the transcript and join two rows by hand.
 *
 * The tool is the label and the command is the finding, which is why the
 * command is the one thing here at full contrast.
 */
export const OneCallRefused: Story = {
  args: {
    said: SAID,
    refused: [{ tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" }],
  },
};

/**
 * **What a restart meets, which the rows alone do not say.** This is the state
 * the report came from: refusals listed beside a screen whose one control was
 * *Restart step*, so the list read as the diagnosis and the button read as the
 * cure. It is not one — a drone's toolset is rendered when it spawns, and a
 * restart renders the same one.
 *
 * **A mechanism and not a tip.** It says how the toolset is built and what has
 * to change for it to differ. Nothing here tells a person to do anything, and
 * nothing here is hedged: the allowlist is what it is.
 *
 * The second sentence is here because a `Bash` row is here. Every `Bash` grant
 * is one declared command and there is no other spelling of one, so a refused
 * command is always a command the Manifest does not declare to this step.
 */
export const WhatARestartMeets: Story = {
  args: {
    said: SAID,
    again: `${AGAIN} ${DECLARED}`,
    refused: [
      { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
      { tool: "Bash", detail: "cargo nextest run --package ipc" },
      { tool: "Bash", detail: "cargo test -p ipc detail" },
    ],
  },
};

/**
 * **Nothing a Manifest declares was refused, so no Manifest is named.**
 * `WebFetch` is a capability Armada grants nowhere, and `commands` is a section
 * that cannot widen it — a sentence sending a person there would send them to
 * edit a file that cannot help them.
 *
 * What survives is the half that is true of every refusal: the toolset is fixed
 * at spawn, and a restart builds the same one. That is a dead end honestly
 * drawn rather than a route invented to avoid one.
 */
export const NothingAManifestDeclares: Story = {
  args: {
    said: SAID,
    again: AGAIN,
    refused: [
      { tool: "WebFetch", detail: "https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html" },
      { tool: "WebSearch", detail: "tokio mutex poisoning" },
    ],
  },
};

/**
 * Several, oldest first — **the earliest refusals and not the last**. What
 * stopped a drone is what it reached for before it started working around being
 * stopped, so reading down the list reads forwards in time.
 *
 * The tool column is sized once for the whole list, so `Bash` and `WebFetch` do
 * not step the commands in and out down the page.
 */
export const SeveralCallsRefused: Story = {
  args: {
    said: SAID,
    refused: [
      { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
      { tool: "Bash", detail: "mkdir -p xtask/src/rules_tests" },
      { tool: "WebFetch", detail: "https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html" },
      { tool: "Write", detail: "xtask/src/rules_tests/refused.rs" },
    ],
  },
};

/**
 * **The same command five times, as five rows.** Collapsing them to one row
 * with a count was considered and refused: a drone denied the same thing five
 * times is a drone that did not learn, and that is the most diagnostic thing on
 * this screen. One row with `×5` beside it reads as a single denial.
 */
export const TheSameCallRefusedAgain: Story = {
  args: {
    said: SAID,
    refused: Array.from({ length: 5 }, () => ({
      tool: "Bash",
      detail: "cargo nextest run --package ipc 2>&1 | tail -80",
    })),
  },
};

/**
 * **A command longer than the row, wrapped and not clipped.** Fleet bounds the
 * argument at 200 characters with whitespace collapsed, so this is the widest
 * row that can arrive.
 *
 * An ellipsis was the alternative and it takes away what the row is for: the
 * tail of a command is the part that gets pasted into an allowlist. Nothing
 * here is laid out to a width, so no row scrolls the panel sideways — narrow
 * the canvas and the command reflows under its own left edge.
 */
export const ACommandLongerThanTheRow: Story = {
  args: {
    said: SAID,
    refused: [
      {
        tool: "Bash",
        detail:
          "cat <<'EOF' > xtask/src/rules_tests/refused.rs use std::path::Path; " +
          "use crate::Report; pub fn every_refusal_names_its_command(root: &Path) -> Report { let " +
          "mut report = Report::new(\"every refu",
      },
      {
        tool: "Read",
        detail:
          "crates/fleet/src/transcript/backfill_of_a_stopped_job_with_a_very_long_path/refusals.rs",
      },
    ],
  },
};

/**
 * **A command longer than the wire carries, said on the row.** Fleet stops at
 * 200 characters and a heredoc does not, so the row shows what fits and states
 * how much there was.
 *
 * **It sits on its own line and never trails the command.** Set inside the mono
 * block it would be selected and copied with the command it is a caveat about,
 * and a cut command pasted into an allowlist as a whole one is the failure this
 * exists to prevent. A tooltip is worse still: it is not there at the moment
 * the command is being copied.
 *
 * The sentence is the list note's, one level down — *showing 200 of 14,320
 * characters* under a command, *showing 50 of 137 refused calls* under the
 * list. The last row is a transcript written before Fleet recorded a size: it
 * says it was cut and claims no total, because `200 characters shown` reads as
 * the whole of it.
 */
export const ACommandCutByTheWire: Story = {
  args: {
    said: SAID,
    refused: [
      {
        tool: "Bash",
        detail:
          "cat <<'EOF' > xtask/src/rules_tests/refused.rs use std::path::Path; " +
          "use crate::Report; pub fn every_refusal_names_its_command(root: &Path) -> Report { let " +
          "mut report = Report::new(\"every refu",
        size: "showing 200 of 14,320 characters",
      },
      { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
      {
        tool: "Write",
        detail: "packages/components/src/compositions/Refusals/Refusals.tsx",
        size: "cut at 200 characters",
      },
    ],
  },
};

/**
 * **The list is short and what happened was not.** *Showing 50 of 137* is a
 * size rather than a warning: a capped list read as the whole one is worse than
 * no list, and the count travels beside the rows on the wire for exactly this.
 *
 * Fleet caps at fifty. Six rows here, because the argument the note makes is
 * the note, not the scroll.
 */
export const MoreThanTheListHolds: Story = {
  args: {
    said: SAID,
    note: "showing 50 of 137 refused calls",
    refused: [
      { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
      { tool: "Bash", detail: "cargo nextest run --package ipc" },
      { tool: "Bash", detail: "cargo test -p ipc detail" },
      { tool: "Bash", detail: "mkdir -p xtask/src/rules_tests" },
      { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
      { tool: "WebFetch", detail: "https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html" },
    ],
  },
};

/**
 * **The rare refusal the harness explained.** `decision_reason` was empty on
 * every `permission_denied` line observed, so a row led by the reason draws a
 * column of blanks — which is why the command leads and the reason sits under
 * it, in sans, on the rows that have one.
 *
 * Nothing fills a reason in from the trigger. A row with none says nothing
 * rather than repeating the policy the person already read.
 */
export const AReasonTheHarnessGave: Story = {
  args: {
    said: SAID,
    refused: [
      {
        tool: "Bash",
        detail: "rm -rf target",
        because: "rm is not on this drone's allowlist",
      },
      { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
    ],
  },
};

/**
 * **A refusal whose command is not in the record.** The tool and the command
 * are two transcript rows joined by a call id, and a transcript that begins
 * mid-run — an adopted drone whose earlier turns went into a pipe with no
 * reader — has the refusal and not the call.
 *
 * The row says so. A `Bash` row naming no command is the defect this component
 * exists to close, and drawing a bare tool name here would be that defect in a
 * new place.
 */
export const ACommandThatWasNotRecorded: Story = {
  args: {
    said: SAID,
    refused: [
      { tool: "Bash", detail: "" },
      { tool: "Bash", detail: "cargo nextest run --package ipc 2>&1 | tail -80" },
    ],
  },
};

/**
 * **Nothing was refused, so nothing is drawn** — not a heading over a blank,
 * and not an empty list. This canvas is empty on purpose: most stopped jobs
 * were refused nothing, and a section that appeared on every one of them would
 * say a policy was involved where none was.
 */
export const NothingWasRefused: Story = {
  args: {
    said: SAID,
    refused: [],
  },
};
