import type { Meta, StoryObj } from "@storybook/react-vite";
import { FactChip } from "./FactChip";

const meta: Meta<typeof FactChip> = {
  title: "Compositions/Fact chip",
  component: FactChip,
};
export default meta;

type Story = StoryObj<typeof FactChip>;

/**
 * A short value, which is what a chip is for. `not run` is two words wide and
 * the chip is two words wide — the table cell it replaced was the width of its
 * column whatever it held.
 */
export const AShortValue: Story = {
  args: { children: "not run" },
};

/**
 * The five facts a running step carries in the drawing, side by side. Every
 * one of them is a measurement, so every one of them is neutral.
 */
export const TheFactsOnARunningStep: Story = {
  render: () => (
    <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
      <FactChip>3 files · +94 −31</FactChip>
      <FactChip>not run</FactChip>
      <FactChip>2 criteria</FactChip>
      <FactChip>test</FactChip>
      <FactChip>as it was at 14:20</FactChip>
    </div>
  ),
};

/**
 * The verdicts, which are the only chips that take a hue — and they take it
 * per fact. A step with a refused first attempt and an advanced second says
 * both, in the order they happened, rather than summing to one colour.
 */
export const AVerdictPerFact: Story = {
  render: () => (
    <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap" }}>
      <FactChip named="refused">refused · reducer changed</FactChip>
      <FactChip named="advanced">advanced</FactChip>
      <FactChip named="passed">2 of 2 passed</FactChip>
      <FactChip named="not_met">1 of 2 refused</FactChip>
      <FactChip named="waiting">on you · 2m 04s</FactChip>
    </div>
  ),
};

/**
 * A value wider than the column it is in. The chip clips and the whole value
 * stays in the title, which is the same rule the Check's output path follows.
 *
 * **A path does not belong here** — clipping from the right is what leaves a
 * column of rows all reading `packages/settings/src/…`. `PathChip` is the one
 * for that, and it truncates the other end.
 */
export const WiderThanItsColumn: Story = {
  render: () => (
    <div style={{ width: "calc(var(--space-12) * 3)" }}>
      <FactChip title="3 files · +94 −31 · all inside the plan">
        3 files · +94 −31 · all inside the plan
      </FactChip>
    </div>
  ),
};
