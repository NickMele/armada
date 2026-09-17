import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent } from "storybook/test";

import { ACTION } from "../../generated/actions";
import { StudioCapture, type StudioCaptureProps } from "./StudioCapture";

/**
 * The layer over a window, so each story draws a box in the canvas the way the
 * app draws one over a control. The binding is the registry's own.
 */
const BINDING = [...(ACTION.capture_note?.shortcut ?? "")];

const meta: Meta<typeof StudioCapture> = {
  title: "Compositions/Studio capture",
  component: StudioCapture,
  args: {
    note: "",
    onNote: fn(),
    onSave: fn(),
    onCancel: fn(),
    aim: "Stale counts",
    aimed: true,
    binding: BINDING,
  },
};
export default meta;

type Story = StoryObj<typeof StudioCapture>;

const CHIP = { x: 120, y: 120, width: 96, height: 28 };

/** Pointing: the element under the pointer is outlined and named. */
export const Pointing: Story = {
  args: { hovered: { box: CHIP, named: "FilterChip" } },
};

/** Held: the card sits against the element, and the note is written in it. */
export const Writing: Story = {
  args: {
    held: { box: CHIP, chain: "FilterChip ← Board · button.armada-chip" },
    note: "The chip keeps its count after the filter is cleared",
  },
};

/** Out to Fleet. The card waits rather than taking a second press. */
export const Capturing: Story = {
  args: { ...Writing.args, saving: true },
};

/** Fleet did not take it, and the card says so without losing what was typed. */
export const Refused: Story = {
  args: { ...Writing.args, refused: "Fleet is not running, so nothing was captured." },
};

/**
 * No Studio is open, so there is nothing to capture onto and the bar says so
 * rather than offering a Capture that would fail.
 */
export const NothingAimedAt: Story = {
  args: {
    held: { box: CHIP, chain: "FilterChip ← Board · button.armada-chip" },
    note: "The chip keeps its count",
    aim: "No Studio open — open one on Studios to capture onto it",
    aimed: false,
  },
};

/**
 * **Capture is refused until something is said, and until a Studio is aimed
 * at.** A blank note would be a pin with nothing on it, and one with nowhere to
 * land would fail after the person had typed it — neither is a press to offer.
 * Escape closes the card without capturing.
 */
export const WhatTheCardRefuses: Story = {
  render: function Draft(args: StudioCaptureProps) {
    const [note, setNote] = useState("");
    return <StudioCapture {...args} note={note} onNote={setNote} />;
  },
  args: { held: { box: CHIP, chain: "FilterChip ← Board · button.armada-chip" } },
  play: async ({ args, canvas }) => {
    const capture = canvas.getByRole("button", { name: "Capture" });
    await expect(capture).toBeDisabled();

    await userEvent.type(canvas.getByLabelText("Note"), "   ");
    await expect(capture).toBeDisabled();

    await userEvent.type(canvas.getByLabelText("Note"), "the count is stale");
    await expect(capture).toBeEnabled();

    await userEvent.keyboard("{Escape}");
    await expect(args.onCancel).toHaveBeenCalled();
    await expect(args.onSave).not.toHaveBeenCalled();
  },
};
