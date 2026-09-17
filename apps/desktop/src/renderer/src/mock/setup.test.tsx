// Setup on the Manifest surface, through `App` — Journey 3 over a storefront
// nobody set up. Moved here from `Screens/Setup`'s stories, which drew the
// surface from a copy of `App`'s wiring — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { MANIFEST_ID, repository } from "@armada/screens/src/fixtures/build/base";

import { endShownIn, pressable, scrollerOf } from "./scrolled";
import { SCRATCH, SHEET_READ, VERIFY_ENDED, settingUp } from "./setup-fleet";
import type { SettingUp } from "./setup-fleet";
import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The Manifest surface by the rail, and its Set up workspaces tab. */
async function setup(options: SettingUp = {}) {
  mount(settingUp(options));
  await page.getByRole("button", { name: "Manifest", exact: true }).click();
  await page.getByRole("tab", { name: "Set up workspaces" }).click();
  const list = page.getByRole("region", { name: "Workspaces" });
  await expect.element(list).toBeVisible();
  return list;
}

/** A workspace's proposal, opened from the picker, in place to be pressed. */
async function open(list: ReturnType<typeof page.getByRole>, dir: string) {
  await list.getByRole("button", { name: `Open ${dir}` }).click();
  const sheet = page.getByRole("dialog", { name: `Proposal for ${dir}` });
  await entered(sheet);
  return sheet;
}

test("the picker lists every workspace Scan found, ticked by evidence, with the batch's gap", async () => {
  const list = await setup();
  for (const dir of [".", "apps/web", "services/api", "docs"]) {
    await expect.element(list.getByRole("listitem", { name: dir, exact: true })).toBeVisible();
  }
  const docs = list.getByRole("listitem", { name: "docs", exact: true });
  await expect.element(docs.getByRole("checkbox")).not.toBeChecked();
  await expect.element(docs.getByText("no checks proposed")).toBeVisible();
  await expect.element(list.getByRole("listitem", { name: ".", exact: true }).getByText("already set up")).toBeVisible();
  const grid = page.getByRole("region", { name: "Check names across the batch" });
  expect(grid.getByText("missing").elements()).toHaveLength(1);
});

test("a line is read for where it came from, corrected, moved, and the sheet closes back to the picker", async () => {
  const list = await setup();
  const sheet = await open(list, "apps/web");
  const lint = sheet.getByRole("list", { name: "Checks" }).getByRole("listitem", { name: "lint" });
  await expect.element(lint.getByText("apps/web/package.json scripts.lint")).toBeVisible();
  await expect.element(lint.getByText("convention")).toBeVisible();

  await lint.getByRole("button", { name: "Edit lint command" }).click();
  // The field alone: "lint command" is also inside the name of the Edit button it replaces.
  const field = lint.getByRole("textbox", { name: "lint command", exact: true });
  await field.clear();
  await userEvent.type(field, "pnpm eslint . --max-warnings 0{Enter}");
  await expect.element(lint.getByText("edited during setup")).toBeVisible();
  expect(lint.getByText("convention").query()).toBeNull();

  const dev = sheet.getByRole("list", { name: "Commands" }).getByRole("listitem", { name: "dev" });
  await dev.getByRole("button", { name: "Move to Checks" }).click();
  await expect.element(sheet.getByRole("list", { name: "Checks" }).getByRole("listitem", { name: "dev" })).toBeVisible();

  await sheet.getByRole("button", { name: "Back to the workspaces" }).click();
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();
  await expect.element(list.getByRole("listitem", { name: "apps/web" }).getByText("ready to write")).toBeVisible();
  const api = await open(list, "services/api");
  await expect.element(api.getByRole("list", { name: "Ports" }).getByText("compose.yaml services.db.ports")).toBeVisible();
});

test("what runs first and what is destructive are corrected, and Write puts both down", async () => {
  const onWritten = vi.fn();
  const list = await setup({ write: "took", onWritten });
  const sheet = await open(list, "services/api");

  const checkTest = sheet.getByRole("list", { name: "Checks" }).getByRole("listitem", { name: "test" });
  await checkTest.getByRole("button", { name: "Add runs first: test" }).click();
  await checkTest.getByRole("dialog", { name: "test runs first" }).getByRole("checkbox", { name: "migrate" }).click();
  await expect.element(checkTest.getByText("edited during setup")).toBeVisible();
  await expect.element(checkTest.getByRole("button", { name: "Edit test runs first" })).toHaveTextContent("migrate →");
  await checkTest.getByRole("button", { name: "Edit test runs first" }).click();
  await expect.poll(() => checkTest.getByRole("dialog").query()).toBeNull();

  const commands = sheet.getByRole("list", { name: "Commands" });
  const reset = commands.getByRole("listitem", { name: "reset" });
  await reset.getByRole("button", { name: "Mark destructive: reset" }).click();
  const flag = reset.getByRole("dialog", { name: "reset destructive" });
  await expect.element(flag.getByText(/this is your judgement/)).toBeVisible();
  // The input sits under the switch it styles, so the press goes to the input itself.
  // A press focuses what it presses; a DOM click alone does not.
  const destructive = flag.getByRole("switch", { name: /Destructive/ }).element() as HTMLElement;
  destructive.focus();
  destructive.click();
  await expect.element(reset.getByText("edited during setup")).toBeVisible();
  await expect.element(reset.getByRole("button", { name: "Edit reset destructive" })).toHaveTextContent(/^destructive$/);
  await expect.element(reset.getByRole("switch", { name: /Destructive/ })).toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => reset.getByRole("dialog").query()).toBeNull();
  await expect.element(page.getByRole("dialog", { name: "Proposal for services/api" })).toBeVisible();

  const migrate = commands.getByRole("listitem", { name: "migrate" });
  await migrate.getByRole("button", { name: "Mark destructive: migrate" }).click();
  await expect.element(migrate.getByRole("switch", { name: /Destructive/ })).toBeDisabled();
  await expect.element(migrate.getByText("test runs it first, so it cannot be destructive.")).toBeVisible();
  await migrate.getByRole("button", { name: "Mark destructive: migrate" }).click();
  await expect.poll(() => migrate.getByRole("dialog").query()).toBeNull();

  await sheet.getByRole("button", { name: "Write services/api/armada.yml" }).click();
  await expect.element(sheet.getByText(/^Wrote services\/api\/armada.yml at /)).toBeVisible();
  expect(onWritten).toHaveBeenCalledTimes(1);
  const [file, text] = onWritten.mock.calls[0] as [string, string];
  expect(file).toBe("services/api/armada.yml");
  expect(text).toContain("  test:\n    run: pnpm test\n    requires:\n      - migrate\n");
  expect(text).toContain("  reset:\n    run: pnpm reset\n    destructive: true\n");
});

test("both policies read in words, each with what the selected value does", async () => {
  const list = await setup();
  const sheet = await open(list, "apps/web");
  const policy = sheet.getByRole("region", { name: "Policy" });
  await expect.element(policy.getByRole("radio", { name: "A person merges never" })).toBeChecked();
  await expect.element(policy.getByRole("radio", { name: "Fleet merges once the forge's checks pass checks-pass" })).not.toBeChecked();
  await expect.element(policy.getByText("A person merges every pull request here.")).toBeVisible();
  (policy.getByRole("radio", { name: "Fleet merges whatever ran always" }).element() as HTMLElement).click();
  await expect.element(policy.getByText("Fleet merges without waiting on the forge's checks.")).toBeVisible();
});

test("Write, and the sheet that wrote a workspace's file offers Verify for it, sending its workspace", async () => {
  const onVerify = vi.fn();
  const list = await setup({ write: "took", sheet: SHEET_READ, onVerify });
  const sheet = await open(list, "apps/web");
  await sheet.getByRole("button", { name: "Write apps/web/armada.yml" }).click();
  await expect.element(sheet.getByText(/^Wrote apps\/web\/armada.yml at /)).toBeVisible();
  expect(sheet.getByRole("button", { name: /^Edit / }).query()).toBeNull();
  await sheet.getByRole("region", { name: "Verify" }).getByRole("button", { name: "Verify" }).click();
  expect(onVerify).toHaveBeenCalledWith("apps/web");
});

test("a workspace's Verify lands on the sheet that wrote its file, and not on another's", async () => {
  const list = await setup({
    write: "took",
    sheet: { state: "read", sheet: { setup: [], checks: [], commands: [], verify: { ...VERIFY_ENDED, workspace: "apps/web" } } },
  });
  const web = await open(list, "apps/web");
  await web.getByRole("button", { name: "Write apps/web/armada.yml" }).click();
  await expect.element(web.getByRole("region", { name: "Verify" }).getByText(/^Ran 4 of 4\./)).toBeVisible();

  await web.getByRole("button", { name: "Back to the workspaces" }).click();
  const api = await open(list, "services/api");
  await api.getByRole("button", { name: "Write services/api/armada.yml" }).click();
  const theirs = api.getByRole("region", { name: "Verify" });
  await expect.element(theirs.getByRole("button", { name: "Verify" })).toBeEnabled();
  expect(theirs.getByText(/^Ran 4 of 4\./).query()).toBeNull();
});

test("a root with an armada.yml already offers no Write, and sends a person to the Edit tab", async () => {
  const list = await setup();
  const sheet = await open(list, ".");
  await expect.element(sheet.getByText("armada.yml is already here")).toBeVisible();
  expect(sheet.getByRole("button", { name: "Write armada.yml" }).query()).toBeNull();
  await sheet.getByRole("button", { name: "Open the Edit tab" }).click();
  await expect.element(page.getByRole("tab", { name: "Edit", selected: true })).toBeVisible();
});

test("a root nobody set up lands on Verify after Write", async () => {
  const list = await setup({ write: "took", rootSetUp: false, sheet: SHEET_READ });
  const sheet = await open(list, ".");
  await sheet.getByRole("button", { name: "Write armada.yml" }).click();
  await expect.element(sheet.getByRole("region", { name: "Verify" }).getByRole("button", { name: "Verify" })).toBeEnabled();
});

test("Write refused: the fault is under the row it names, and nothing was written", async () => {
  const list = await setup({ write: "refused" });
  const sheet = await open(list, "apps/web");
  await sheet.getByRole("button", { name: "Write apps/web/armada.yml" }).click();
  const lint = sheet.getByRole("list", { name: "Checks" }).getByRole("listitem", { name: "lint" });
  await expect.element(lint.getByText("is empty.")).toBeVisible();
  await expect.element(sheet.getByText("Not written")).toBeVisible();
});

test("a file already there: nothing written over it, and the picker says so", async () => {
  const list = await setup({ write: "appeared" });
  const sheet = await open(list, "apps/web");
  await sheet.getByRole("button", { name: "Write apps/web/armada.yml" }).click();
  await expect.element(sheet.getByText("Nothing was written over it. It reads:")).toBeVisible();
  expect(sheet.getByRole("button", { name: /^Write / }).query()).toBeNull();
  await sheet.getByRole("button", { name: "Back to the workspaces" }).click();
  await expect.element(list.getByRole("listitem", { name: "apps/web" }).getByText("already set up")).toBeVisible();
});

const TWO: SettingUp = { repositories: [repository(), SCRATCH], sheet: SHEET_READ };

test("a set-up and a not-set-up repository: both listed, the one not set up grouped as such", async () => {
  mount(settingUp(TWO));
  await page.getByRole("button", { name: MANIFEST_ID }).click();
  await expect.element(page.getByRole("menuitem", { name: MANIFEST_ID })).toBeInTheDocument();
  const menu = page.getByRole("menu");
  await expect.element(menu.getByText("Not set up")).toBeVisible();
  await expect.element(menu.getByRole("menuitem", { name: "scratch" })).toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
  expect(page.getByRole("region", { name: "Workspaces" }).query()).toBeNull();
});

test("picking one not set up opens Setup alone; a workspace verifies before the root is set up", async () => {
  const onVerify = vi.fn();
  mount(settingUp({ ...TWO, onVerify }));
  await page.getByRole("button", { name: MANIFEST_ID }).click();
  await page.getByRole("menuitem", { name: "scratch" }).click();
  const list = page.getByRole("region", { name: "Workspaces" });
  const sheet = await open(list, "apps/web");
  await sheet.getByRole("button", { name: "Write apps/web/armada.yml" }).click();
  const verify = sheet.getByRole("region", { name: "Verify" });
  await expect.element(verify.getByRole("button", { name: "Verify" })).toBeVisible();
  expect(verify.getByText(/at its root/).query()).toBeNull();
  await verify.getByRole("button", { name: "Verify" }).click();
  expect(onVerify).toHaveBeenCalledWith("apps/web");
});

test("pick one not set up, write its root, and it reads as set up with an Edit tab", async () => {
  mount(settingUp(TWO));
  await page.getByRole("button", { name: MANIFEST_ID }).click();
  await page.getByRole("menuitem", { name: "scratch" }).click();
  const list = page.getByRole("region", { name: "Workspaces" });
  await expect.element(list).toBeVisible();
  expect(page.getByRole("tab", { name: "Edit" }).query()).toBeNull();
  const sheet = await open(list, ".");
  await sheet.getByRole("button", { name: "Write armada.yml" }).click();
  await expect.element(sheet.getByRole("region", { name: "Verify" }).getByRole("button", { name: "Verify" })).toBeEnabled();
  // The sheet covers the rail while it is up, so it closes before the picker is pressed.
  await sheet.getByRole("button", { name: "Back to the workspaces" }).click();
  await page.getByRole("button", { name: "scratch" }).click();
  const menu = page.getByRole("menu");
  await expect.element(menu.getByRole("menuitem", { name: "scratch" })).toBeInTheDocument();
  expect(menu.getByText("Not set up").query()).toBeNull();
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("tab", { name: "Edit" })).toBeVisible();
});

test("a batch taller than the window scrolls under tabs that stay put, and a proposal scrolls on its own", async () => {
  const list = await setup({ more: 16 });
  const batch = page.getByRole("region", { name: "Check names across the batch" }).element();
  const picker = scrollerOf(batch);
  expect(picker).not.toBeNull();
  expect(scrollerOf(list.element())).toBe(picker);
  expect(picker!.scrollHeight).toBeGreaterThan(picker!.clientHeight);
  expect(endShownIn(batch, picker!)).toBe(false);
  const tabsAt = page.getByRole("tablist").element().getBoundingClientRect().top;

  picker!.scrollTop = picker!.scrollHeight;
  await expect.poll(() => endShownIn(batch, picker!)).toBe(true);
  expect(page.getByRole("tablist").element().getBoundingClientRect().top).toBe(tabsAt);
  const last = page.elementLocator(batch).getByRole("button", { name: "packages/pkg-16" }).elements();
  expect(pressable(last[0]!)).toBe(true);

  await userEvent.click(last[0]!);
  const sheet = page.getByRole("dialog", { name: "Proposal for packages/pkg-16" });
  await entered(sheet);
  const own = scrollerOf(sheet.getByRole("list", { name: "Checks" }).element());
  expect(own).not.toBeNull();
  expect(own).not.toBe(picker);
  expect(sheet.element().contains(own) || own!.contains(sheet.element())).toBe(true);
  await sheet.getByRole("button", { name: "Back to the workspaces" }).click();
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();
});
