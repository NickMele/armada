import type { Meta, StoryObj } from "@storybook/react-vite";

import { Tokens } from "./Tokens";

/**
 * The design tokens, as specimens.
 *
 * **Every value on these pages is read off the running stylesheet**, so a
 * specimen cannot claim a colour the app does not use. The names are read off
 * `packages/tokens/src/` too — a token added there and not here shows as a gap
 * in the group rather than as nothing at all.
 *
 * The rules these illustrate are in `docs/contracts/design-system.md`. A
 * specimen shows what a value looks like; the contract says when to reach for
 * it, and the two are not the same document.
 */
const meta = {
  title: "Foundations/Tokens",
  component: Tokens,
} satisfies Meta<typeof Tokens>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ground: Story = {
  args: {
    label: "Ground",
    note: "Deep desaturated blue-slate rather than near-black: an instrument panel rather than a terminal, and room for status colour to sit without vibrating.",
    names: [
  "bg-base",
  "bg-sunken",
  "bg-raised",
  "bg-overlay",
  "bg-hover",
  "border-subtle",
  "border-default",
  "border-strong",
],
  },
};

export const Foreground: Story = {
  args: {
    label: "Foreground",
    note: "Four weights of text on the ground above, and the one that goes on a solid accent fill.",
    names: [
  "fg-default",
  "fg-muted",
  "fg-subtle",
  "fg-inverse",
],
    as: "fill",
  },
};

export const Accent: Story = {
  args: {
    label: "Accent",
    note: "One accent, and what focus is drawn in. A second accent is a second meaning nobody defined.",
    names: [
  "accent",
  "accent-hover",
  "accent-muted",
],
  },
};

export const Status: Story = {
  args: {
    label: "Status",
    note: "Derived from the Job state machine, one to one, and never chosen. Rejected and killed are decisions a person made, not failures, and must not read as errors.",
    names: [
  "status-not-started",
  "status-running",
  "status-awaiting-review",
  "status-escalated",
  "status-completed-success",
  "status-completed-failed",
  "status-rejected",
  "status-killed",
  "status-piloted",
  "status-awaiting-attestation",
  "status-superseded",
  "status-awaiting-approval",
],
  },
};

export const BelowJobLevel: Story = {
  args: {
    label: "Below Job level",
    note: "Hue below a Job is permitted only where it is declared, and every value is aliased from its Job counterpart so the mapping is stated rather than borrowed.",
    names: [
  "step-advanced",
  "step-running",
  "step-waiting",
  "step-failed",
  "step-failed-bg",
  "step-stopped-bg",
  "verdict-met",
  "verdict-not-met",
],
  },
};

export const TypeScale: Story = {
  args: {
    label: "Type scale",
    note: "Set in the face the app sets it in. The sample is the value applied, not a description of it.",
    names: [
  "text-2xs",
  "text-xs",
  "text-sm",
  "text-base",
  "text-lg",
  "text-xl",
  "text-2xl",
],
    as: "text",
  },
};

export const TypeSettings: Story = {
  args: {
    label: "Line height, weight, tracking and family",
    names: [
  "weight-body",
  "weight-medium",
  "weight-heading",
  "tracking-caps",
],
    as: "fill",
  },
};

export const Spacing: Story = {
  args: {
    label: "Spacing",
    note: "Each bar is drawn that wide, so the ratios between steps are the thing on screen rather than a column of numbers.",
    names: [
  "space-1",
  "space-2",
  "space-3",
  "space-4",
  "space-6",
  "space-8",
  "space-12",
],
    as: "size",
  },
};

export const Shape: Story = {
  args: {
    label: "Shape and measure",
    note: "Radii, control heights and the widths a layout is built from.",
    names: [
  "border-width",
  "radius-sm",
  "radius-md",
  "radius-lg",
  "h-row",
  "h-row-header",
  "h-row-action",
  "h-row-stacked",
  "h-control",
  "h-control-sm",
  "h-badge",
  "h-kbd",
  "h-menu-item",
  "h-status-bar",
  "pad-card",
  "pad-row-stacked",
  "sidebar-default",
  "sidebar-min",
  "sidebar-max",
  "sidebar-rail",
  "w-run-column",
  "w-dialog",
  "w-sheet",
  "w-dialog-wide",
  "w-menu",
  "w-tooltip",
  "w-toast",
],
    as: "size",
  },
};

export const Motion: Story = {
  args: {
    label: "Motion",
    names: [
  "duration-fast",
  "duration-base",
  "ease",
  "duration-pulse",
],
    as: "duration",
  },
};
