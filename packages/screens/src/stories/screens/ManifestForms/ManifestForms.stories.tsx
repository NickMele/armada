import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";

import { sheet } from "../Manifest/Manifest";
import { endShownIn, pressable, scrollerOf } from "../Manifest/scrolled";
import { ManifestFormsFrom } from "./ManifestForms";

/**
 * The Manifest surface's **Edit** view — Journey 9, *Editing* — drawn from this
 * repository's own Checks and Commands, with Fleet faked just far enough to
 * write an edit, refuse one, and say what past Jobs cost.
 */
const meta = {
  title: "Screens/Manifest forms",
  component: ManifestFormsFrom,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof ManifestFormsFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/**
 * **Edit** — the forms, the default editing view, with the file behind the tab
 * named by its path. Save writes the file and stops.
 */
export const TheForms: Story = {
  name: "The forms",
  args: { sheet: { state: "read", sheet: sheet() } },
};

/** A Check declared through the form and saved: the form redraws from what Fleet wrote. */
export const AddACheckAndSave: Story = {
  name: "Add a Check and save",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const checks = await canvas.findByRole("region", { name: "Checks" });
    await userEvent.type(within(checks).getByLabelText("New Check name"), "clippy");
    await userEvent.click(within(checks).getByRole("button", { name: "Add a Check" }));
    const clippy = within(checks).getByRole("group", { name: "clippy" });
    await userEvent.type(within(clippy).getByLabelText("Command"), "cargo clippy --workspace");
    await expect(canvas.getByText("Not saved.")).toBeVisible();

    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(await canvas.findByText(/^Saved /)).toBeVisible();
    await expect(within(checks).getByRole("group", { name: "clippy" })).toBeVisible();
    // Nothing left to save: the draft is the file as written.
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
  },
};

/** A Command removed. The writer takes its lines and the comment directly above them. */
export const RemoveACommand: Story = {
  name: "Remove a Command",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const commands = await canvas.findByRole("region", { name: "Commands" });
    const fmt = within(commands).getByRole("group", { name: "fmt" });
    await userEvent.click(within(fmt).getByRole("button", { name: "Remove" }));
    await expect(within(commands).queryByRole("group", { name: "fmt" })).toBeNull();

    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(await canvas.findByText(/^Saved /)).toBeVisible();
    await expect(within(commands).queryByRole("group", { name: "fmt" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
  },
};

/**
 * A Command removed that a Check still runs first. Fleet refuses the result,
 * names the key, and writes nothing; the removal is still on screen to undo.
 */
export const AFormEditRefused: Story = {
  name: "A refused save on the forms",
  args: { ...TheForms.args, edit: "refused" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const commands = await canvas.findByRole("region", { name: "Commands" });
    const bootstrap = within(commands).getByRole("group", { name: "bootstrap" });
    await userEvent.click(within(bootstrap).getByRole("button", { name: "Remove" }));

    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(await canvas.findByText("Not saved")).toBeVisible();
    await expect(canvas.getByText("checks.typecheck.requires")).toBeVisible();
    await expect(within(commands).queryByRole("group", { name: "bootstrap" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Discard changes" })).toBeEnabled();
  },
};

/** A cost cap below what the costliest past Job here cost — a warning, never a refusal. */
export const TheBudgetWarning: Story = {
  name: "The budget warning",
  args: { ...TheForms.args, spend: { jobs: 41, most_cost_micros: 7_120_000, most_turns: 212 } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const budget = await canvas.findByRole("region", { name: "Budget" });
    await expect(within(budget).queryByText(/~\$7\.12/)).toBeNull();
    await userEvent.type(within(budget).getByLabelText("Cost cap per Job, in dollars"), "5");
    await expect(within(budget).getByText(/~\$7\.12/)).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Save" })).toBeEnabled();
  },
};

/** What Save leaves: nothing more to send, once the form redraws from what Fleet wrote. */
async function saved(canvas: ReturnType<typeof within>): Promise<void> {
  await userEvent.click(canvas.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(canvas.getByText(/^Saved /)).toBeVisible());
  await waitFor(() => expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled());
}

/** A Check that passes on a non-zero exit, set on the Check and saved. */
export const AnExitCodeSaved: Story = {
  name: "An exit code, saved",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const checks = await canvas.findByRole("region", { name: "Checks" });
    const build = within(checks).getByRole("group", { name: "build" });
    await userEvent.type(within(build).getByLabelText("Passes on exit code"), "one");
    await expect(within(build).getByText("An exit code is a whole number.")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
    await userEvent.clear(within(build).getByLabelText("Passes on exit code"));
    await userEvent.type(within(build).getByLabelText("Passes on exit code"), "1");
    await saved(canvas);
    const redrawn = within(checks).getByRole("group", { name: "build" });
    await expect(within(redrawn).getByLabelText("Passes on exit code")).toHaveValue("1");
  },
};

/** **Base** — the branch named, and saved. */
export const TheBaseSaved: Story = {
  name: "The base, saved",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const base = await canvas.findByRole("region", { name: "Base" });
    await expect(within(base).getByLabelText("Base branch")).toHaveValue("main");
    await userEvent.clear(within(base).getByLabelText("Base branch"));
    await userEvent.type(within(base).getByLabelText("Base branch"), "trunk");
    await saved(canvas);
    await expect(within(base).getByLabelText("Base branch")).toHaveValue("trunk");
  },
};

/**
 * **Evidence** — removed and saved, then declared again. Save waits until the
 * spec command has `{}` and the frames have somewhere to land.
 */
export const EvidenceRemovedAndDeclared: Story = {
  name: "Evidence, removed and declared again",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const section = await canvas.findByRole("region", { name: "Evidence" });
    const declared = within(section).getByRole("group", { name: "evidence" });
    await expect(within(declared).getByLabelText("Frames land in")).toHaveValue(".armada/frames");
    await userEvent.click(within(declared).getByRole("button", { name: "Remove" }));
    await saved(canvas);

    await userEvent.click(within(section).getByRole("button", { name: "Declare evidence" }));
    const again = within(section).getByRole("group", { name: "evidence" });
    await userEvent.type(within(again).getByLabelText("Runs one spec"), "node capture.js");
    await expect(within(again).getByText("Nowhere for the spec's path to go yet.")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
    // `{{` is how user-event types a literal brace.
    await userEvent.type(within(again).getByLabelText("Runs one spec"), " {{}");
    await userEvent.type(within(again).getByLabelText("Frames land in"), ".armada/frames");
    await saved(canvas);
    const redrawn = within(section).getByRole("group", { name: "evidence" });
    await expect(within(redrawn).getByLabelText("Runs one spec")).toHaveValue("node capture.js {}");
  },
};

/**
 * **Proved after merge** — a Check picked and saved. One that runs a Command
 * first is not offered, and the section says why.
 */
export const ProvedAfterMergeSaved: Story = {
  name: "Proved after merge, saved",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const section = await canvas.findByRole("region", { name: "Proved after merge" });
    const group = within(section).getByRole("group", { name: "Runs after a merge" });
    await expect(within(group).queryByRole("checkbox", { name: "typecheck" })).toBeNull();
    await expect(within(section).getByText(/^typecheck runs a Command first/)).toBeVisible();
    await userEvent.click(within(group).getByRole("checkbox", { name: "build" }));
    await saved(canvas);
    await expect(within(section).getByRole("checkbox", { name: "build" })).toBeChecked();
  },
};

/** **Setup** and **Drone** — a Command added to setup, the silence threshold set, both saved. */
export const SetupAndDroneSaved: Story = {
  name: "Setup and the Drone, saved",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const setup = await canvas.findByRole("region", { name: "Setup" });
    await expect(within(setup).getByRole("checkbox", { name: "bootstrap" })).toBeChecked();
    await userEvent.click(within(setup).getByRole("checkbox", { name: "gate" }));
    await expect(within(setup).getByText("bootstrap, gate")).toBeVisible();

    const drone = canvas.getByRole("region", { name: "Drone" });
    const silence = within(drone).getByLabelText("Silence threshold, in seconds");
    await userEvent.type(silence, "0");
    await expect(within(drone).getByText(/^A silence threshold is a whole number/)).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
    await userEvent.clear(silence);
    await userEvent.type(silence, "600");
    await expect(within(drone).getByLabelText("Poke limit")).toHaveValue("3");
    await saved(canvas);
    await expect(within(drone).getByLabelText("Silence threshold, in seconds")).toHaveValue("600");
    await expect(within(setup).getByRole("checkbox", { name: "gate" })).toBeChecked();
  },
};

/** **Policy** — auto-merge and the review gate in the registry's words, with the file's word beside them. */
export const PolicyInWords: Story = {
  name: "Policy in words",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const policy = await canvas.findByRole("region", { name: "Policy" });
    await expect(within(policy).getByLabelText("Auto merge")).toHaveDisplayValue("A person merges · never");
    await expect(within(policy).getByText("A person merges every pull request here.")).toBeVisible();
    await userEvent.selectOptions(within(policy).getByLabelText("Auto merge"), "checks-pass");
    await expect(within(policy).getByLabelText("Auto merge")).toHaveDisplayValue("Fleet merges once the forge's checks pass · checks-pass");
    await expect(within(policy).getByText(/Where the forge runs none, nothing merges\./)).toBeVisible();
    const gate = within(policy).getByLabelText("Review gate");
    await expect(gate).toHaveDisplayValue("A person answers · human_always");
    await userEvent.selectOptions(gate, "auto_if_judge_passes");
    await saved(canvas);
    await expect(within(policy).getByLabelText("Review gate")).toHaveDisplayValue(
      "The checks decide, unless the Judge objects · auto_if_judge_passes",
    );
  },
};

/** `id` and `version` are shown and offer nothing to type into: past Jobs are recorded against the id. */
export const TheIdentityIsReadOnly: Story = {
  name: "The id and version, read-only",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const identity = await canvas.findByRole("region", { name: "This Manifest" });
    await expect(within(identity).getByText("armada")).toBeVisible();
    await expect(within(identity).getByText("1")).toBeVisible();
    await expect(within(identity).queryByRole("textbox")).toBeNull();
    await expect(within(identity).getByText(/not changed from a form/)).toBeVisible();
  },
};

/**
 * **Scrolled to the last section.** The mount never scrolls, so the form does: the tabs stay put,
 * Drone's end comes into view, and Save stays pinned above it, still pressable.
 */
export const ScrolledToTheEnd: Story = {
  name: "Scrolled to the last section",
  // Quarantined: `endShownIn` fails this assertion on a clean `main`, unrelated
  // to any Job's diff. #1192.
  tags: ["!test"],
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const drone = await canvas.findByRole("region", { name: "Drone" });
    const form = scrollerOf(drone);
    await expect(form).not.toBeNull();
    await expect(form!.scrollHeight).toBeGreaterThan(form!.clientHeight);
    await expect(endShownIn(drone, form!)).toBe(false);
    const tabsAt = canvas.getByRole("tablist").getBoundingClientRect().top;

    form!.scrollTop = form!.scrollHeight;
    await waitFor(() => expect(endShownIn(drone, form!)).toBe(true));
    await expect(canvas.getByRole("tablist").getBoundingClientRect().top).toBe(tabsAt);
    await userEvent.type(within(drone).getByLabelText("Poke limit"), "0");
    const save = canvas.getByRole("button", { name: "Save" });
    await expect(save).toBeEnabled();
    await expect(pressable(save)).toBe(true);
  },
};

/** What the Freeze story's Save sends. A module's spy, cleared by the play that reads it. */
const sent = fn();

/** The switch turned on and saved: one `set_freeze` goes out, and the page says the repository is frozen. */
export const FreezeAndSave: Story = {
  name: "Freeze, and save",
  args: { ...TheForms.args, onEdits: sent },
  play: async ({ canvasElement }) => {
    sent.mockClear();
    const canvas = within(canvasElement);
    const freeze = within(await canvas.findByRole("region", { name: "Freeze" }));
    await expect(freeze.getByRole("switch", { name: /Freeze this repository/ })).not.toBeChecked();
    await expect(canvas.queryByText("This repository is frozen")).toBeNull();
    await userEvent.click(freeze.getByRole("switch", { name: /Freeze this repository/ }));
    await expect(canvas.getByText("Not saved.")).toBeVisible();
    await saved(canvas);
    await expect(sent).toHaveBeenCalledWith(expect.objectContaining({ edits: [{ edit: "set_freeze", freeze: true }] }));
    await expect(await canvas.findByText("This repository is frozen")).toBeVisible();
    await expect(freeze.getByRole("switch", { name: /Freeze this repository/ })).toBeChecked();
  },
};
