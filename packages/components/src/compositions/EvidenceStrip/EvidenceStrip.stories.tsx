import type { Meta, StoryObj } from "@storybook/react-vite";
import { FileDiff } from "lucide-react";
import { expect } from "storybook/test";
import { EvidenceStrip, type EvidenceChip } from "./EvidenceStrip";

/**
 * One story per state: everything a finished step produced, the strip with the
 * viewer's previous artifact parked in it, and a running step where the list is
 * not finished and the label says so.
 */
const meta: Meta<typeof EvidenceStrip> = {
  title: "Compositions/Evidence strip",
  component: EvidenceStrip,
};
export default meta;

type Story = StoryObj<typeof EvidenceStrip>;

const produced: EvidenceChip[] = [
  { id: "sym", says: "public symbols 41 → 41", kind: "measurement" },
  { id: "bench", says: "bench 1.42 → 1.19µs", kind: "measurement" },
  { id: "diff", says: "−318 +94 · 5 files", kind: "code diff", icon: FileDiff },
  { id: "migration", says: "MIGRATION.md", kind: "document" },
];

/**
 * Everything else the step produced. **The kind under each value is what says
 * which viewer is behind it** — the chips are deliberately one shape, so
 * nothing but the word distinguishes a measurement from a patch.
 */
export const AlsoProduced: Story = {
  args: { chips: produced, label: "also produced" },
};

/**
 * A check's output has taken the viewer, and the artifact that was showing is
 * parked in the strip, first and marked.
 *
 * **Nothing was destroyed.** One press puts the default view back — without
 * that the page loses the opinion it was composed with after three presses.
 */
export const WithTheDefaultViewParked: Story = {
  args: {
    label: "also produced",
    openId: "assertions",
    chips: [
      { id: "assertions", says: "assertion set", kind: "the default view", named: "not_met" },
      ...produced,
    ],
  },
};

/**
 * A running step. **`so far` rather than `also produced`** — the tense is the
 * caller's, because only the caller knows whether the list is finished, and a
 * strip that read `also produced` mid-run would claim a step was done.
 */
export const SoFar: Story = {
  args: {
    label: "so far",
    chips: [
      { id: "req", says: "POST /v1/jobs → 422", kind: "request & response" },
      { id: "queries", says: "queries 3 → 3", kind: "measurement" },
    ],
  },
};

/**
 * Which artifact the viewer is showing is on the chip, not only in the
 * stylesheet.
 *
 * **What earns the assertion is that selection here is a border colour.** Every
 * chip is otherwise the same shape, so nothing in the rendering told a reader
 * who was not looking at it which of five artifacts was open.
 */
export const TheOpenChipSaysSo: Story = {
  args: { chips: produced, openId: "bench", label: "also produced" },
  play: async ({ canvas }) => {
    const open = canvas.getByRole("button", { pressed: true });
    await expect(open).toHaveTextContent("bench 1.42 → 1.19µs");
    await expect(canvas.getAllByRole("button", { pressed: false })).toHaveLength(3);
  },
};
