import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { LocateForm, type LocateFormProps } from "./LocateForm";

/**
 * Locate, as the app draws it: a folder to add, or the quieter clone. Controlled, so each story
 * is one moment; `Screens/Locate` presses them in order.
 */
const meta: Meta<typeof LocateForm> = {
  title: "Compositions/Locate form",
  component: LocateForm,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof LocateForm>;

const BASE: LocateFormProps = {
  open: true,
  mode: "folder",
  path: "",
  url: "",
  parent: "",
  landsIn: null,
  sending: false,
  ready: false,
  onMode: fn(),
  onPath: fn(),
  onUrl: fn(),
  onParent: fn(),
  onChoose: fn(),
  onSend: fn(),
  onCancel: fn(),
};

/** Nothing typed: the field names what it is, and Add waits for a path. */
export const AFolder: Story = {
  name: "A folder",
  args: BASE,
  play: async ({ args, canvasElement, userEvent }) => {
    const dialog = within(canvasElement).getByRole("dialog", { name: "Add a repository" });
    await expect(within(dialog).getByLabelText("Project location")).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: /^Add repository/ })).toBeDisabled();
    await userEvent.click(within(dialog).getByRole("button", { name: "Choose a folder" }));
    await expect(args.onChoose).toHaveBeenCalledWith("path");
    // Escape is the way out, and it sends nothing.
    await userEvent.keyboard("{Escape}");
    await expect(args.onCancel).toHaveBeenCalled();
    await expect(args.onSend).not.toHaveBeenCalled();
  },
};

/** A relative path is refused where it is typed, before Fleet is asked. */
export const NotAFullPath: Story = {
  name: "Not a full path",
  args: { ...BASE, path: "code/scratch", pathMessage: "A full path, starting with /." },
};

/** The clone names the folder it lands in before anything is pressed. */
export const CloneFromAURL: Story = {
  name: "Clone from a URL",
  args: {
    ...BASE,
    mode: "clone",
    url: "https://github.com/owner/storefront.git",
    parent: "/Users/user/code",
    landsIn: "/Users/user/code/storefront",
    ready: true,
  },
};

/** Underway: the dialog says so, every field holds still, and Clone cannot be pressed again. */
export const CloneUnderway: Story = {
  name: "Clone underway",
  args: { ...CloneFromAURL.args, sending: true },
  play: async ({ args, canvasElement, userEvent }) => {
    const dialog = within(canvasElement).getByRole("dialog", { name: "Add a repository" });
    await expect(within(dialog).getByText(/Git is cloning into/)).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: /^Clone repository/ })).toBeDisabled();
    await userEvent.keyboard("{Enter}");
    await expect(args.onSend).not.toHaveBeenCalled();
  },
};

/** Git's own words, under Fleet's code. */
export const CloneRefused: Story = {
  name: "Clone refused",
  args: {
    ...CloneFromAURL.args,
    refusal: {
      code: "fleet.clone_refused",
      saying:
        "git refused the clone: fatal: repository 'https://github.com/owner/storefront.git/' not found",
    },
  },
};

/** A destination already there: Fleet's words, and where to go from them. */
export const DestinationNotEmpty: Story = {
  name: "Destination not empty",
  args: {
    ...CloneFromAURL.args,
    refusal: {
      code: "fleet.destination_occupied",
      saying: "/Users/user/code/storefront already exists and is not empty",
      next: "Choose another folder to clone into.",
    },
  },
};
