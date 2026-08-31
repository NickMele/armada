import type { Meta, StoryObj } from "@storybook/react-vite";

import type { DebugPayload } from "../ErrorNotice/payload";
import { FileAnIssue, FilingReview } from "./FileAnIssue";
import { envelopeOf, NOT_OFFERED } from "./issue";

/**
 * The same refusal the error notice's stories carry, so the two surfaces can be
 * read against each other: what the expanded view shows, and what leaves if
 * somebody files it.
 */
const REFUSED: DebugPayload = {
  code: "judge.undecided",
  message: "judge returned prose for criterion 2",
  run_id: "01JQ8ZC4M2WYVK7T3RQN8H",
  job_id: "job_31c7",
  drone_id: "drn_4c8",
  step_id: "verify",
  fields: [
    { key: "criterion", value: "2" },
    { key: "judge_model", value: "sonnet" },
    { key: "response_bytes", value: "1184" },
  ],
  chain: [
    "judge: no verdict parsed from response",
    "gate verify: undecided",
    "job_31c7: escalated",
  ],
  bridgeProtocol: "5.2",
  fleetProtocol: "5.2",
  at: "2026-08-30T09:16:40Z",
};

/**
 * The one item every filing carries, built the way Bridge builds it rather than
 * spelled out here — a fixture that wrote its own envelope would be a second
 * answer to what the required row says.
 */
const ENVELOPE = envelopeOf(REFUSED);

const meta: Meta<typeof FileAnIssue> = {
  title: "Errors/File an issue",
  component: FileAnIssue,
};
export default meta;

type Story = StoryObj<typeof FileAnIssue>;

/**
 * What ships: one item, and it cannot be taken out.
 *
 * **A review with one locked row is still a review**, because the thing it
 * stands between is a press and a public tracker. It puts the exact text on
 * screen, states that nothing was scrubbed, and says Armada opens nothing — and
 * it takes a second, deliberate press to produce anything at all.
 *
 * The drawing gave the envelope no read-this mark, on the grounds that it is
 * structured and bounded. It is not: the structured fields are, and `message`
 * and `chain` are prose written by whatever raised the error. So the row that
 * cannot be removed is the row that most needs reading, and it carries the
 * warning every row carries.
 */
/** A diff row, so the removable form is visible rather than only described. */
const DIFF = {
  id: "worktree-diff",
  label: "What the drone changed",
  warning:
    "A patch is the contents of files on this machine, including any the drone read a secret out of.",
  body: [
    "diff --git a/crates/judge/src/parse.rs b/crates/judge/src/parse.rs",
    "@@ -18,7 +18,7 @@",
    "-    let verdict = line.split_once(':')?.1;",
    "+    let verdict = line.split_once(':').map(|pair| pair.1)?;",
  ].join("\n"),
};

/**
 * The control, at rest.
 *
 * **This is all one press does.** It opens the review; nothing is composed and
 * nothing is copied until a second, deliberate press. Ghost, no glyph, no kbd —
 * the error treatment's rules for every control on it.
 */
export const TheControl: Story = {
  args: {
    compose: () => ({ title: REFUSED.message, attached: [ENVELOPE], withheld: NOT_OFFERED }),
  },
};

/**
 * What ships: one item, and it cannot be taken out.
 *
 * **A review with one locked row is still a review**, because what it stands
 * between is a press and a public tracker. It puts the exact text on screen,
 * states that nothing was scrubbed, and says Armada opens nothing.
 *
 * The drawing gave the envelope no read-this mark, on the grounds that it is
 * structured and bounded. It is not: the structured fields are, and `message`
 * and `chain` are prose written by whatever raised the error. So the row that
 * cannot be removed carries the same warning as every other, and the warning is
 * the payload's own sentence pair rather than one written for this dialog.
 */
export const OneItemAndItCannotBeRemoved: Story = {
  render: () => (
    <FilingReview
      offered={{ title: REFUSED.message, attached: [ENVELOPE], withheld: NOT_OFFERED }}
      removed={new Set()}
      onToggle={() => undefined}
      onCancel={() => undefined}
      onCopy={() => undefined}
    />
  ),
};

/**
 * What the review does when there is something to decide.
 *
 * **The second row is a fixture and nothing in Bridge supplies one.** The
 * drawing named four more items — a transcript, a judge response, a diff and a
 * doctor report — and at the two placements this control appears on, no Bridge
 * failure holds any of them: doctor is not built, a judge response and a diff
 * belong to a Job read whole, and the transcript is blocked on an open
 * question. The row is here so the removable form is visible, and so the gap is
 * a thing somebody can see rather than a sentence in a report.
 */
export const AnItemThatCanBeRemoved: Story = {
  render: () => (
    <FilingReview
      offered={{ title: REFUSED.message, attached: [ENVELOPE, DIFF], withheld: NOT_OFFERED }}
      removed={new Set()}
      onToggle={() => undefined}
      onCancel={() => undefined}
      onCopy={() => undefined}
    />
  ),
};

/**
 * The same review with the removable row taken out.
 *
 * The row stays on screen unchecked rather than disappearing — what was decided
 * about is as much a part of the review as what is going, and a row that
 * vanished would leave nothing saying a choice had been made.
 */
export const AnItemRemoved: Story = {
  render: () => (
    <FilingReview
      offered={{ title: REFUSED.message, attached: [ENVELOPE, DIFF], withheld: NOT_OFFERED }}
      removed={new Set([DIFF.id])}
      onToggle={() => undefined}
      onCancel={() => undefined}
      onCopy={() => undefined}
    />
  ),
};
