import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { ManifestForm, type ManifestFormDraft } from "./ManifestForm";

/**
 * The Manifest as forms — Journey 9, *Editing* — drawn from lines in this
 * repository's own `armada.yml`, in the shape the Manifest surface holds a
 * draft.
 *
 * **The policy words are Fleet's**, handed across with the value, so nothing
 * here spells a registry of its own.
 */
const meta: Meta<typeof ManifestForm> = {
  title: "Compositions/Manifest form",
  component: ManifestForm,
};
export default meta;

type Story = StoryObj<typeof ManifestForm>;

const noop = () => {};

const DRAFT: ManifestFormDraft = {
  checks: [
    {
      name: "build",
      run: "cargo build --workspace --locked",
      requires: [],
      when: "",
      narrow: { run: "cargo build --locked", each: "-p {}", from: "", under: "crates", except: "" },
    },
    { name: "bridge_test", run: "pnpm bridge-test", requires: ["bootstrap"], when: "packages/**\napps/**", narrow: null },
  ],
  commands: [
    { name: "bootstrap", run: "pnpm install --frozen-lockfile", destructive: false, serve: "", ready: "", links: [] },
    {
      name: "storybook_dev",
      run: "",
      destructive: false,
      serve: "pnpm -C packages/components storybook --port ${port.storybook}",
      ready: "curl -sf http://localhost:${port.storybook}",
      links: [{ url: "http://localhost:${port.storybook}", name: "Storybook" }],
    },
  ],
  ports: [{ name: "storybook", container: "", env: "STORYBOOK_PORT" }],
  autoMerge: "never",
  reviewGate: "human_always",
  costCap: "5",
  turnCap: "300",
};

const BASE = {
  path: "/Users/user/armada/armada.yml",
  draft: DRAFT,
  onDraft: noop,
  autoMergeWords: ["never", "checks-pass", "always"],
  reviewGateWords: ["human_always", "auto_if_judge_passes"],
  problems: {},
  budgetWarnings: [],
  changed: false,
  onSave: noop,
  onDiscard: noop,
};

/** Nothing touched: Save and Discard wait for a change. */
export const AtRest: Story = { name: "At rest", args: BASE };

/** A change not yet saved. */
export const Changed: Story = {
  name: "Changed",
  args: { ...BASE, changed: true, receipt: "Not saved." },
};

/** Fleet refused a result that would not load, and said which key. The draft stays. */
export const Refused: Story = {
  name: "Refused",
  args: {
    ...BASE,
    changed: true,
    receipt: "Not saved.",
    refused: {
      saying: "armada.yml would not load after these edits: 1 fault",
      faults: [{ key: "checks.bridge_test.requires", fault: "names `browsers`, which no Command declares" }],
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("checks.bridge_test.requires")).toBeVisible();
  },
};

/** A cap below what the costliest past Job here cost. A warning, never a refusal. */
export const BelowAPastJob: Story = {
  name: "Below a past Job",
  args: {
    ...BASE,
    draft: { ...DRAFT, costCap: "5" },
    changed: true,
    budgetWarnings: [
      "The costliest of this repository's 41 past Jobs cost ~$7.12, more than this cap. A Job like it would stop before it finished.",
    ],
  },
};
