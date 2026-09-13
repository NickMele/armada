import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { repository } from "../../../fixtures/build/base";
import { CHOSEN, LocateFrom } from "./Locate";

/**
 * Locate — Journey 3's *Getting in* — from the rail's **Add a repository**: a folder added, or a
 * clone from a URL, landing on Setup for what Fleet now serves. Nothing native opens: the folder
 * dialog answers `/Users/user/scratch`.
 */
const meta = {
  title: "Screens/Locate",
  component: LocateFrom,
  parameters: { layout: "fullscreen" },
  args: { repositories: [repository()], onAdded: fn(), onCloned: fn() },
} satisfies Meta<typeof LocateFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

const URL = "https://github.com/owner/storefront.git";

async function opened(canvasElement: HTMLElement) {
  const canvas = within(canvasElement);
  await userEvent.click(canvas.getByRole("button", { name: "Add a repository" }));
  return { canvas, dialog: canvas.getByRole("dialog", { name: "Add a repository" }) };
}

async function cloneFrom(canvasElement: HTMLElement) {
  const { canvas, dialog } = await opened(canvasElement);
  await userEvent.click(within(dialog).getByRole("button", { name: "Clone from a URL" }));
  await userEvent.type(within(dialog).getByLabelText("Repository URL"), URL);
  await userEvent.type(within(dialog).getByLabelText("Clone into"), "/Users/user/code");
  // Named before anything is pressed, as Fleet will name it.
  const lands = within(dialog).getByRole("group", { name: "Project location" });
  await expect(within(lands).getByText("/Users/user/code/storefront")).toBeVisible();
  await userEvent.click(within(dialog).getByRole("button", { name: /^Clone repository/ }));
  return { canvas, dialog };
}

/** A folder chosen and added: the dialog closes, the picker holds it, and Setup is open for it. */
export const AFolderAdded: Story = {
  name: "A folder added",
  play: async ({ args, canvasElement }) => {
    const { canvas, dialog } = await opened(canvasElement);
    await userEvent.click(within(dialog).getByRole("button", { name: "Choose a folder" }));
    await expect(within(dialog).getByLabelText("Project location")).toHaveValue(CHOSEN);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Add repository/ }));
    await expect(args.onAdded).toHaveBeenCalledWith(CHOSEN);
    await waitFor(() => expect(canvas.queryByRole("dialog", { name: "Add a repository" })).toBeNull());
    const picker = canvas.getByRole("combobox", { name: "Project" });
    await expect(picker).toHaveValue(CHOSEN);
    await expect(within(within(picker).getByRole("group", { name: "Not set up" })).getByRole("option", { name: "scratch" })).toBeInTheDocument();
    await expect(await canvas.findByRole("region", { name: "Workspaces" })).toBeVisible();
  },
};

/** A clone underway says so, and a second press — the button or Enter — sends nothing. */
export const ACloneUnderway: Story = {
  name: "A clone underway",
  args: { clone: "underway" },
  play: async ({ args, canvasElement }) => {
    const { dialog } = await cloneFrom(canvasElement);
    await expect(await within(dialog).findByText(/Git is cloning into/)).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: /^Clone repository/ })).toBeDisabled();
    await userEvent.keyboard("{Enter}");
    await expect(args.onCloned).toHaveBeenCalledTimes(1);
    await expect(args.onCloned).toHaveBeenCalledWith(URL, "/Users/user/code");
  },
};

/** Git's refusal reads in the dialog under Fleet's code, and nothing moves behind it. */
export const ACloneRefused: Story = {
  name: "A clone refused",
  args: { clone: "refused" },
  play: async ({ canvasElement }) => {
    const { canvas, dialog } = await cloneFrom(canvasElement);
    const refusal = await within(dialog).findByRole("alert");
    await expect(within(refusal).getByText("fleet.clone_refused")).toBeVisible();
    await expect(within(refusal).getByText(/git refused the clone: fatal: repository/)).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: /^Clone repository/ })).toBeEnabled();
    await expect(canvas.getByRole("combobox", { name: "Project" })).toHaveValue("/Users/user/armada");
  },
};

/** A destination already full: Fleet's words, and where to go from them. */
export const ANonEmptyDestination: Story = {
  name: "A non-empty destination",
  args: { clone: "occupied" },
  play: async ({ canvasElement }) => {
    const { dialog } = await cloneFrom(canvasElement);
    const refusal = await within(dialog).findByRole("alert");
    await expect(within(refusal).getByText("fleet.destination_occupied")).toBeVisible();
    await expect(within(refusal).getByText("/Users/user/code/storefront already exists and is not empty")).toBeVisible();
    await expect(within(refusal).getByText("Choose another folder to clone into.")).toBeVisible();
  },
};

/** Escape closes it with nothing sent, and it opens again empty. */
export const ClosedWithNothingDone: Story = {
  name: "Closed with nothing done",
  play: async ({ args, canvasElement }) => {
    const { canvas, dialog } = await opened(canvasElement);
    await userEvent.type(within(dialog).getByLabelText("Project location"), "/Users/user/elsewhere");
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog", { name: "Add a repository" })).toBeNull();
    await expect(args.onAdded).not.toHaveBeenCalled();
    const again = await opened(canvasElement);
    await expect(within(again.dialog).getByLabelText("Project location")).toHaveValue("");
    await userEvent.click(within(again.dialog).getByRole("button", { name: /^Cancel/ }));
    await expect(canvas.queryByRole("dialog", { name: "Add a repository" })).toBeNull();
  },
};

const SET_UP = { ...repository(), root: "/Users/user/code/web-app", manifest: { ...repository().manifest!, id: "storefront" } };
const LOOSE = { root: "/Users/user/scratch", records_root: "/records/scratch" };
const API = { root: "/Users/user/code/api", records_root: "/records/api" };
const OLD_API = { root: "/Users/user/old/api", records_root: "/records/old-api" };

/** The picker names a Manifest by its id and a folder nobody set up by its name; two alike take their parent. */
export const ThePickersNames: Story = {
  name: "The picker's names",
  args: { repositories: [SET_UP, LOOSE, API, OLD_API] },
  play: async ({ canvasElement }) => {
    const picker = within(canvasElement).getByRole("combobox", { name: "Project" });
    const setUp = within(picker).getByRole("option", { name: "storefront" });
    await expect(setUp).toHaveAttribute("title", SET_UP.root);
    const notSetUp = within(within(picker).getByRole("group", { name: "Not set up" }));
    await expect(notSetUp.getByRole("option", { name: "scratch" })).toHaveAttribute("title", LOOSE.root);
    await expect(notSetUp.getByRole("option", { name: "code/api" })).toHaveAttribute("title", API.root);
    await expect(notSetUp.getByRole("option", { name: "old/api" })).toHaveAttribute("title", OLD_API.root);
    await expect(within(picker).queryByRole("option", { name: "web-app" })).toBeNull();
  },
};
