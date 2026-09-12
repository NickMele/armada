import type { Meta, StoryObj } from "@storybook/react-vite";

import { WorkGroups } from "./WorkGroups";

/**
 * A step's work, with runs of one tool folded to a line.
 *
 * **The rows are the caller's.** Every story here puts plain lines in `body` so
 * the folding is what is on trial; in the panel those are the activity log's own
 * rows, with their payloads and their keyboard.
 */
const meta = {
  title: "Compositions/Work groups",
  component: WorkGroups,
  parameters: { layout: "padded" },
} satisfies Meta<typeof WorkGroups>;

export default meta;
type Story = StoryObj<typeof meta>;

/** A line standing in for a log row. */
function row(text: string) {
  return <p style={{ margin: 0, padding: "var(--space-1) var(--space-3)" }}>{text}</p>;
}

/** Nine reads behind one line, and a reply drawn as itself. */
export const AFoldedRun: Story = {
  name: "A folded run",
  args: {
    emptyNote: "Nothing yet",
    groups: [
      {
        id: "1",
        name: "Read",
        mono: true,
        meta: "9 calls · 142ms",
        folded: true,
        body: (
          <>
            {row("Read  crates/store/src/read.rs")}
            {row("Read  crates/store/src/write.rs")}
          </>
        ),
      },
      { id: "2", name: "", body: row("I have the shape of it now.") },
    ],
  },
};

/**
 * A run holding a failure, which is never folded.
 *
 * **The whole of the rule.** A count in front of the thing that went wrong is
 * the reading a person opened the panel to avoid.
 */
export const AFailureIsNeverFolded: Story = {
  name: "A failure is never folded",
  args: {
    emptyNote: "Nothing yet",
    groups: [
      { id: "1", name: "Bash", mono: true, meta: "3 calls · 2s", folded: true, body: row("Bash  cargo fmt") },
      {
        id: "2",
        name: "",
        body: (
          <>
            {row("Bash  cargo nextest run")}
            {row("The call failed")}
          </>
        ),
      },
    ],
  },
};

/** The rows this Bridge has no drawing for, counted by the wire's own kind. */
export const CountedRows: Story = {
  name: "Rows it does not draw",
  args: {
    emptyNote: "Nothing yet",
    groups: [{ id: "1", name: "", body: row("A Drone run opened") }],
    unread: [
      { kind: "system/thinking_tokens", count: 757 },
      { kind: "the Drone's reasoning, not carried", count: 216 },
    ],
  },
};

/** No rows at all, which is ordinary and never an error. */
export const Empty: Story = {
  name: "Nothing yet",
  args: { groups: [], emptyNote: "This step has not opened a Drone yet." },
};
