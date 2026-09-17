// The Manifest surface's Edit view, through `App` — Journey 9, *Editing* —
// drawn from this repository's own Checks and Commands. Moved here from
// `Screens/Manifest forms`' stories — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { manifesting } from "./manifest-fleet";
import type { Manifesting } from "./manifest-fleet";
import { endShownIn, pressable, scrollerOf } from "./scrolled";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The Manifest surface by the rail, on its Edit tab. */
async function forms(options: Manifesting = {}): Promise<void> {
  mount(manifesting(options));
  await page.getByRole("button", { name: "Manifest", exact: true }).click();
  await page.getByRole("tab", { name: "Edit" }).click();
}

const save = () => page.getByRole("button", { name: "Save", exact: true });

/** What Save leaves: nothing more to send, once the form redraws from what Fleet wrote. */
async function saved(): Promise<void> {
  await save().click();
  await expect.element(page.getByText(/^Saved /)).toBeVisible();
  await expect.element(save()).toBeDisabled();
}

/** A control that is an input under what styles it: focused and pressed as a press would. */
function press(element: Element): void {
  (element as HTMLElement).focus();
  (element as HTMLElement).click();
}

test("a Check declared through the form and saved redraws from what Fleet wrote", async () => {
  await forms();
  const checks = page.getByRole("region", { name: "Checks" });
  await userEvent.type(checks.getByLabelText("New Check name"), "clippy");
  await checks.getByRole("button", { name: "Add a Check" }).click();
  const clippy = checks.getByRole("group", { name: "clippy" });
  await userEvent.type(clippy.getByLabelText("Command"), "cargo clippy --workspace");
  await expect.element(page.getByText("Not saved.")).toBeVisible();
  await saved();
  await expect.element(checks.getByRole("group", { name: "clippy" })).toBeVisible();
});

test("a Command removed and saved stays removed", async () => {
  await forms();
  const commands = page.getByRole("region", { name: "Commands" });
  await commands.getByRole("group", { name: "fmt" }).getByRole("button", { name: "Remove" }).click();
  expect(commands.getByRole("group", { name: "fmt" }).query()).toBeNull();
  await saved();
  expect(commands.getByRole("group", { name: "fmt" }).query()).toBeNull();
});

test("a refused save names the key, writes nothing, and leaves the removal to undo", async () => {
  await forms({ edit: "refused" });
  const commands = page.getByRole("region", { name: "Commands" });
  await commands.getByRole("group", { name: "bootstrap" }).getByRole("button", { name: "Remove" }).click();
  await save().click();
  await expect.element(page.getByText("Not saved", { exact: true })).toBeVisible();
  await expect.element(page.getByText("checks.typecheck.requires")).toBeVisible();
  expect(commands.getByRole("group", { name: "bootstrap" }).query()).toBeNull();
  await expect.element(page.getByRole("button", { name: "Discard changes" })).toBeEnabled();
});

test("a cost cap below the costliest past Job here warns, and never refuses", async () => {
  await forms({ spend: { jobs: 41, most_cost_micros: 7_120_000, most_turns: 212 } });
  const budget = page.getByRole("region", { name: "Budget" });
  await expect.element(budget).toBeVisible();
  expect(budget.getByText(/~\$7\.12/).query()).toBeNull();
  await userEvent.type(budget.getByLabelText("Cost cap per Job, in dollars"), "5");
  await expect.element(budget.getByText(/~\$7\.12/)).toBeVisible();
  await expect.element(save()).toBeEnabled();
});

test("an exit code must be a whole number, and one that is saves", async () => {
  await forms();
  const build = page.getByRole("region", { name: "Checks" }).getByRole("group", { name: "build" });
  const code = build.getByLabelText("Passes on exit code");
  await userEvent.type(code, "one");
  await expect.element(build.getByText("An exit code is a whole number.")).toBeVisible();
  await expect.element(save()).toBeDisabled();
  await code.clear();
  await userEvent.type(code, "1");
  await saved();
  await expect.element(page.getByRole("region", { name: "Checks" }).getByRole("group", { name: "build" }).getByLabelText("Passes on exit code")).toHaveValue("1");
});

test("the base branch, named and saved", async () => {
  await forms();
  const base = page.getByRole("region", { name: "Base" });
  await expect.element(base.getByLabelText("Base branch")).toHaveValue("main");
  await base.getByLabelText("Base branch").clear();
  await userEvent.type(base.getByLabelText("Base branch"), "trunk");
  await saved();
  await expect.element(base.getByLabelText("Base branch")).toHaveValue("trunk");
});

test("evidence removed and saved, then declared again once the spec has {} and frames a place", async () => {
  await forms();
  const section = page.getByRole("region", { name: "Evidence" });
  const declared = section.getByRole("group", { name: "evidence" });
  await expect.element(declared.getByLabelText("Frames land in")).toHaveValue(".armada/frames");
  await declared.getByRole("button", { name: "Remove" }).click();
  await saved();

  await section.getByRole("button", { name: "Declare evidence" }).click();
  const again = section.getByRole("group", { name: "evidence" });
  await userEvent.type(again.getByLabelText("Runs one spec"), "node capture.js");
  await expect.element(again.getByText("Nowhere for the spec's path to go yet.")).toBeVisible();
  await expect.element(save()).toBeDisabled();
  await userEvent.type(again.getByLabelText("Runs one spec"), " {{}");
  await userEvent.type(again.getByLabelText("Frames land in"), ".armada/frames");
  await saved();
  await expect.element(section.getByRole("group", { name: "evidence" }).getByLabelText("Runs one spec")).toHaveValue("node capture.js {}");
});

test("proved after merge: a Check picked and saved, and one that runs a Command first is not offered", async () => {
  await forms();
  const section = page.getByRole("region", { name: "Proved after merge" });
  const group = section.getByRole("group", { name: "Runs after a merge" });
  await expect.element(group).toBeVisible();
  expect(group.getByRole("checkbox", { name: "typecheck" }).query()).toBeNull();
  await expect.element(section.getByText(/^typecheck runs a Command first/)).toBeVisible();
  press(group.getByRole("checkbox", { name: "build" }).element());
  await saved();
  await expect.element(section.getByRole("checkbox", { name: "build" })).toBeChecked();
});

test("setup and the Drone: a Command added to setup, the silence threshold set, both saved", async () => {
  await forms();
  const setup = page.getByRole("region", { name: "Setup" });
  await expect.element(setup.getByRole("checkbox", { name: "bootstrap" })).toBeChecked();
  press(setup.getByRole("checkbox", { name: "gate" }).element());
  await expect.element(setup.getByText("bootstrap, gate")).toBeVisible();

  const drone = page.getByRole("region", { name: "Drone" });
  const silence = drone.getByLabelText("Silence threshold, in seconds");
  await userEvent.type(silence, "0");
  await expect.element(drone.getByText(/^A silence threshold is a whole number/)).toBeVisible();
  await expect.element(save()).toBeDisabled();
  await silence.clear();
  await userEvent.type(silence, "600");
  await expect.element(drone.getByLabelText("Poke limit")).toHaveValue("3");
  await saved();
  await expect.element(drone.getByLabelText("Silence threshold, in seconds")).toHaveValue("600");
  await expect.element(setup.getByRole("checkbox", { name: "gate" })).toBeChecked();
});

test("policy reads in the registry's words, with the file's word beside them", async () => {
  await forms();
  const policy = page.getByRole("region", { name: "Policy" });
  await expect.element(policy.getByLabelText("Auto merge")).toHaveDisplayValue("A person merges · never");
  await expect.element(policy.getByText("A person merges every pull request here.")).toBeVisible();
  await userEvent.selectOptions(policy.getByLabelText("Auto merge"), "checks-pass");
  await expect.element(policy.getByLabelText("Auto merge")).toHaveDisplayValue("Fleet merges once the forge's checks pass · checks-pass");
  await expect.element(policy.getByText(/Where the forge runs none, nothing merges\./)).toBeVisible();
  await expect.element(policy.getByLabelText("Review gate")).toHaveDisplayValue("A person answers · human_always");
  await userEvent.selectOptions(policy.getByLabelText("Review gate"), "auto_if_judge_passes");
  await saved();
  await expect.element(policy.getByLabelText("Review gate")).toHaveDisplayValue("The checks decide, unless the Judge objects · auto_if_judge_passes");
});

test("the id and version are shown and offer nothing to type into", async () => {
  await forms();
  const identity = page.getByRole("region", { name: "This Manifest" });
  await expect.element(identity.getByText("armada", { exact: true })).toBeVisible();
  await expect.element(identity.getByText("1", { exact: true })).toBeVisible();
  expect(identity.getByRole("textbox").query()).toBeNull();
  await expect.element(identity.getByText(/not changed from a form/)).toBeVisible();
});

test("scrolled to the last section: the tabs stay put, and Save stays pinned and pressable", async () => {
  await forms();
  const droneRegion = page.getByRole("region", { name: "Drone" });
  await expect.element(droneRegion).toBeInTheDocument();
  const drone = droneRegion.element();
  const form = scrollerOf(drone);
  expect(form).not.toBeNull();
  expect(form!.scrollHeight).toBeGreaterThan(form!.clientHeight);
  expect(endShownIn(drone, form!)).toBe(false);
  const tabsAt = page.getByRole("tablist").element().getBoundingClientRect().top;

  form!.scrollTop = form!.scrollHeight;
  await expect.poll(() => endShownIn(drone, form!)).toBe(true);
  expect(page.getByRole("tablist").element().getBoundingClientRect().top).toBe(tabsAt);
  await userEvent.type(droneRegion.getByLabelText("Poke limit"), "0");
  await expect.element(save()).toBeEnabled();
  expect(pressable(save().element())).toBe(true);
});

test("freeze turned on and saved sends one set_freeze, and the page says the repository is frozen", async () => {
  const onEdits = vi.fn();
  await forms({ onEdits });
  const freeze = page.getByRole("region", { name: "Freeze" });
  const toggle = freeze.getByRole("switch", { name: /Freeze this repository/ });
  await expect.element(toggle).not.toBeChecked();
  expect(page.getByText("This repository is frozen").query()).toBeNull();
  press(toggle.element());
  await expect.element(page.getByText("Not saved.")).toBeVisible();
  await saved();
  expect(onEdits).toHaveBeenCalledWith(expect.objectContaining({ edits: [{ edit: "set_freeze", freeze: true }] }));
  await expect.element(page.getByText("This repository is frozen")).toBeVisible();
  await expect.element(toggle).toBeChecked();
});
