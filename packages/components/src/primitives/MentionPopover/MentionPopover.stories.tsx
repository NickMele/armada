import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { MentionPopover, useMention } from "./MentionPopover";
import { Textarea } from "../Textarea/Textarea";

/**
 * The `@` mention popup's own two states: results to pick from, and the
 * honest answer to a miss — `CommandPalette`'s own empty state, on a smaller
 * list. `useMention` in this directory is what decides when the popup opens
 * and what it holds; nothing about that logic is a rendering, so it is not
 * asserted here — see `DispatchRequest`'s own stories for the field it
 * drives.
 */
const meta: Meta<typeof MentionPopover> = {
  title: "Primitives/MentionPopover",
  component: MentionPopover,
  args: {
    query: "com",
    active: 0,
    listId: "mention-list",
    optionId: (index: number) => `mention-option-${index}`,
    onHover: fn(),
    onChoose: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof MentionPopover>;

export const Results: Story = {
  args: {
    results: ["packages/components/src/index.ts", "packages/components/package.json"],
  },
  /** A press on a row is a choice, and it carries the row's own path. */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("option", { name: "packages/components/package.json" }));
    await expect(args.onChoose).toHaveBeenCalledWith("packages/components/package.json");
  },
};

/** The row `Enter` would choose, drawn active without a press. */
export const SecondRowActive: Story = {
  args: {
    results: ["packages/components/src/index.ts", "packages/components/package.json"],
    active: 1,
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("option", { name: "packages/components/package.json" }),
    ).toHaveAttribute("aria-selected", "true");
    await expect(
      canvas.getByRole("option", { name: "packages/components/src/index.ts" }),
    ).toHaveAttribute("aria-selected", "false");
  },
};

/**
 * Nothing matched. **Names the query**, the same honesty `CommandPalette`'s
 * own empty state carries — there is no suggestion and no did-you-mean.
 */
export const NoMatch: Story = {
  args: { query: "zzz", results: [] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("No file matches “zzz”.")).toBeVisible();
  },
};

/** What the harness below searches. Both paths carry `com`. */
const FILES = ["packages/components/src/index.ts", "packages/components/package.json"];

/**
 * A field driven by `useMention`, which is the only way to press a key at the
 * contract the hook holds — the popup itself takes an active row as a prop and
 * cannot move it.
 */
function DrivenField() {
  const [brief, setBrief] = useState("");
  const mention = useMention(brief, setBrief, search);
  return (
    <div className="armada-mention-anchor">
      <Textarea
        label="Brief"
        value={brief}
        {...mention.fieldAria}
        onChange={mention.onFieldChange}
        onKeyDown={mention.onFieldKeyDown}
        onSelect={mention.onFieldSelect}
        onBlur={mention.onFieldBlur}
      />
      {mention.open ? (
        <MentionPopover
          query={mention.query}
          results={mention.results}
          active={mention.active}
          listId={mention.listId}
          optionId={mention.optionId}
          onHover={mention.onHover}
          onChoose={mention.onChoose}
        />
      ) : null}
    </div>
  );
}

async function search(query: string): Promise<readonly string[]> {
  return FILES.filter((path) => path.includes(query));
}

/**
 * The keyboard contract, and the combobox wiring that tells a screen reader
 * any of it happened. **Escape on the empty state is the regression this
 * asserts**: the handler used to return on an empty list before it ever
 * reached Escape, so "No file matches" was the one state a person could not
 * dismiss — they had to delete characters until the popup went away.
 */
export const DrivenByTheHook: Story = {
  name: "Driven by the hook",
  render: () => <DrivenField />,
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByLabelText("Brief");
    await userEvent.click(field);
    await userEvent.type(field, "@com");

    // Open: the field says so, and points at the list it opened.
    await expect(field).toHaveAttribute("aria-expanded", "true");
    await expect(field).toHaveAttribute("aria-controls", canvas.getByRole("listbox").id);

    // The arrow moves the active row, and the field names it.
    const second = canvas.getByRole("option", { name: FILES[1] });
    await userEvent.keyboard("{ArrowDown}");
    await expect(second).toHaveAttribute("aria-selected", "true");
    await expect(field).toHaveAttribute("aria-activedescendant", second.id);

    // Enter takes the active row, not the first one.
    await userEvent.keyboard("{Enter}");
    await expect(field).toHaveValue(`@${FILES[1]} `);

    // The regression: Escape closes the popup while nothing matches.
    await userEvent.type(field, "@zzz");
    await expect(canvas.getByText("No file matches “zzz”.")).toBeVisible();
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByText("No file matches “zzz”.")).toBeNull();
    await expect(field).not.toHaveAttribute("aria-expanded");
  },
};
