// The Studios surface, through `App`, on a Fleet that keeps Studios — #1287's definition of done,
// and #1341's: every scenario keeps Studios, so the surface opens wherever it is reached.
//
// #1287's is:
// open Studios from the rail, start one, drag a node, close Bridge, reopen it read-only with the
// node where it was left, press Continue, and accept a proposed relation, with Helm's footer naming
// the Studio. `crates/acceptance/tests/studio.rs` lists the read-only reopen as proved here.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { mountApp, type Mounted } from "./mount";
import { everyKind, studying } from "./studio-fleet";
import { entered } from "./testing";

const windows: { app: Mounted; host: HTMLElement }[] = [];

/** Open a Bridge window on `fleet`, sharing its `window.armada` where one was open before. */
function open(scenario: Parameters<typeof mountApp>[0], shared?: Mounted["api"]): Mounted {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(scenario, host, shared);
  windows.push({ app, host });
  return app;
}

/** Close the window. Fleet, and every Studio it keeps, outlives it. */
function close(): void {
  for (const one of windows.splice(0)) {
    one.app.unmount();
    one.host.remove();
  }
}

afterEach(close);

const node = (name: RegExp) => page.getByRole("group", { name });
/** The one control every act on what is picked lives behind — #1399. */
const offers = () => page.getByRole("button", { name: "Acts", exact: true });
/** Open it. The trigger toggles, so an open menu is left open. */
async function openActs(): Promise<void> {
  await expect.element(offers()).toBeVisible();
  if (offers().element().getAttribute("aria-expanded") !== "true") await offers().click();
}
/** One act, as it is read in the menu. */
const offered = (name: string) => page.getByRole("menuitem", { name, exact: true });
/** Reach one act: open the control, then press what it offers. */
async function act(name: string): Promise<void> {
  await openActs();
  await offered(name).click();
}
const asked = (name: string) => page.getByRole("dialog").getByRole("button", { name, exact: true });

/**
 * Pick one node, the way React Flow's own keyboard contract does. **Not a pointer click**: a node
 * the board has just drawn passes a visibility check before it has settled, and a click that lands
 * in that window selects nothing — which failed under a full parallel suite and nowhere else.
 */
async function pick(name: RegExp): Promise<void> {
  await expect.element(node(name)).toBeVisible();
  node(name).element().focus();
  await userEvent.keyboard("{Enter}");
}
const centre = (element: Element) => {
  const box = element.getBoundingClientRect();
  return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
};

/** A pointer drag, the way React Flow hears one: down on the node, moves and up on the window. */
function drag(element: Element, dx: number, dy: number): void {
  const from = centre(element);
  const at = (x: number, y: number) => ({ clientX: from.x + x, clientY: from.y + y, button: 0, bubbles: true, view: window });
  element.dispatchEvent(new MouseEvent("mousedown", at(0, 0)));
  for (const step of [0.1, 0.5, 1]) window.dispatchEvent(new MouseEvent("mousemove", at(dx * step, dy * step)));
  window.dispatchEvent(new MouseEvent("mouseup", at(dx, dy)));
}

test("a Studio started, laid out, closed, reopened read-only, continued, and a relation accepted", async () => {
  const fleet = studying([]);
  const first = open(fleet.scenario);

  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await expect.element(page.getByText("No Studios yet.", { exact: false })).toBeVisible();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();
  // Started, so the person's own: no Continue to press.
  expect(page.getByRole("button", { name: "Continue" }).query()).toBeNull();

  const [studio] = fleet.studios();
  fleet.helmProposes(studio!.id);
  await expect.element(node(/^Note: Drag me somewhere/)).toBeVisible();
  const before = fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position;

  drag(node(/^Note: Drag me somewhere/).element(), 160, 120);
  await expect.poll(() => fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position.x).toBeGreaterThan(before.x + 60);
  const left = fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position;
  expect(left.y).toBeGreaterThan(before.y + 30);

  close();
  open(fleet.scenario, first.api);

  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "Untitled Studio", exact: true }).click();
  await expect.element(page.getByText("Read-only", { exact: true })).toBeVisible();
  await expect.element(node(/^Note: Drag me somewhere/)).toBeVisible();
  expect(fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position).toEqual(left);

  // Read-only means read-only: a drag saves nothing, and no act on a relation is offered.
  drag(node(/^Note: Drag me somewhere/).element(), 200, 0);
  await new Promise((resolve) => setTimeout(resolve, 200));
  expect(fleet.studios()[0]!.nodes.find((one) => one.kind === "note")!.position).toEqual(left);
  expect(page.getByRole("button", { name: /^Accept: / }).query()).toBeNull();

  // Helm's footer names the Studio, and the node selected on it.
  await expect.element(page.getByText("Studios · Untitled Studio", { exact: true })).toBeVisible();
  node(/^Note: Drag me somewhere/).element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByText("Studios · Untitled Studio · Note Drag me somewhere selected")).toBeVisible();

  await page.getByRole("button", { name: "Continue" }).click();
  expect(page.getByText("Read-only", { exact: true }).query()).toBeNull();
  await page.getByRole("button", { name: /^Accept: Finding What does the note point at\? answers Note Drag me somewhere/ }).click();
  await expect.poll(() => fleet.studios()[0]!.edges.map((edge) => edge.standing)).toEqual(["accepted"]);
  await expect.poll(() => page.getByRole("button", { name: /^Accept: / }).query()).toBeNull();
});

/**
 * #1364's definition of done: a person opens a new Studio, names it, types a
 * note onto it, pastes a link beside it, and reloads Bridge to find all three
 * where they left them — without asking Helm for any of it.
 */
test("a Studio named by hand, with a note typed and a link pasted, survives a reload", async () => {
  const fleet = studying([]);
  const first = open(fleet.scenario);

  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();

  await page.getByRole("button", { name: "Rename Untitled Studio" }).click();
  await userEvent.keyboard("The Board's legend{Enter}");
  await expect.element(page.getByRole("heading", { name: "The Board's legend" })).toBeVisible();
  await expect.poll(() => fleet.studios()[0]!.named_by).toBe("person");

  // The menu, and the two keys behind it. `N` is the note and `V` the link,
  // read off the registry by the surface that binds them.
  await page.getByRole("button", { name: /Node/ }).click();
  await page.getByRole("menuitem", { name: /Note/ }).click();
  // A single key is suppressed while a field holds focus: the `V` in this note
  // stays in the note rather than opening a Link beside it.
  await userEvent.fill(page.getByLabelText("Note", { exact: true }), "The legend is unreadable in View");
  expect(page.getByLabelText("Link", { exact: true }).query()).toBeNull();
  await page.getByRole("button", { name: "Add note" }).click();
  await expect.element(node(/^Note: The legend is unreadable in View/)).toBeVisible();

  await userEvent.keyboard("V");
  await userEvent.fill(page.getByLabelText("Link", { exact: true }), "docs/contracts/design-system.md");
  await page.getByRole("button", { name: "Keep the link" }).click();
  await expect.element(node(/^Link: docs\/contracts\/design-system\.md/)).toBeVisible();

  // Both are a person's, and neither landed at the origin: a node goes where
  // the person is looking, and two at one point would read as one node.
  const [kept] = fleet.studios();
  expect(kept!.nodes.map((one) => one.added_by)).toEqual(["person", "person"]);
  expect(kept!.nodes[0]!.position).not.toEqual(kept!.nodes[1]!.position);

  close();
  open(fleet.scenario, first.api);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "The Board's legend", exact: true }).click();
  await expect.element(node(/^Note: The legend is unreadable in View/)).toBeVisible();
  await expect.element(node(/^Link: docs\/contracts\/design-system\.md/)).toBeVisible();
});

/**
 * One node picked reaches the same act, the same route and the same
 * confirmation as eighteen — #1411. Two of each was two paths that drifted, and
 * the single one left a captured Note's picture on disk.
 */
test("Delete 1 node confirms, with Cancel first, and takes the node's edges with it", async () => {
  const fleet = studying([]);
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();
  fleet.helmProposes(fleet.studios()[0]!.id);
  await expect.element(node(/^Finding: /)).toBeVisible();

  node(/^Finding: /).element().focus();
  await userEvent.keyboard("{Enter}");
  await act("Delete 1 node");
  // Scaling up, so the press inside it waits for it to land — #1323.
  const confirm = page.getByRole("dialog");
  await entered(confirm);
  await expect.element(page.getByRole("button", { name: "Cancel" })).toHaveFocus();
  // What goes with it is said before the press, the same sentences a selection gets.
  await expect.element(confirm.getByText("1 node goes from this Studio, with the 1 edge on it.")).toBeVisible();
  await confirm.getByRole("button", { name: "Delete 1 node" }).click();

  await expect.poll(() => fleet.studios()[0]!.nodes.map((one) => one.kind)).toEqual(["note"]);
  expect(fleet.studios()[0]!.edges).toEqual([]);
});

test("on All repositories the surface asks for one, since a Studio belongs to one", async () => {
  const fleet = studying();
  open({ ...fleet.scenario, state: { ...fleet.scenario.state, repository: null } });
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await expect.element(page.getByText("Pick a repository to open its Studios")).toBeVisible();
});

/** Studios from the rail, answering the surface's own ask for a repository. */
async function openStudios(): Promise<void> {
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await userEvent.selectOptions(page.getByLabelText("Repository", { exact: true }), repository().root);
}

const FAILED = "This repository's Studios could not be read";

test("every-state keeps Studios, so the surface opens on a list and not a read failure", async () => {
  open("every-state");
  await openStudios();

  await expect.element(page.getByRole("button", { name: "Every kind of node and edge", exact: true })).toBeVisible();
  // Untitled, so the list draws what an unnamed Studio is called.
  await expect.element(page.getByRole("button", { name: "Untitled Studio", exact: true })).toBeVisible();
  expect(page.getByText(FAILED).query()).toBeNull();

  await page.getByRole("button", { name: "Every kind of node and edge", exact: true }).click();
  await expect.element(node(/^Note: The legend under the step bar is unreadable/)).toBeVisible();
  // A Job node reads its state off the Board row this window already holds.
  await expect.element(node(/^Job: /)).toBeVisible();
  // Reopened read-only, so its proposed relations are listed and nothing acts on them.
  await expect.element(page.getByText("Continue to accept or reject.")).toBeVisible();
});

test("a scenario keeping no Studios draws the empty state, not a read failure", async () => {
  open("empty-store");
  await openStudios();

  await expect.element(page.getByText("No Studios yet.", { exact: false })).toBeVisible();
  expect(page.getByText(FAILED).query()).toBeNull();
});

/**
 * #1352's own: the picture a Note kept is drawn on the node, and opened full
 * size beside it. What this holds is the whole path — Fleet's bytes, over the
 * preload, into a `blob:` this window made and an `img` drew — since every
 * piece of it is fine on its own and the frame was invisible all the same.
 */
test("a Note draws the frame it kept, opens it full size, and a Note without one draws no plate", async () => {
  // The `studios` scenario has this repository picked already, and keeps the legend Studio.
  open(studying().scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "The Board's legend", exact: true }).click();

  const kept = node(/^Note: The legend under the step bar is unreadable/);
  await expect.element(kept).toBeVisible();
  // One picture on the board: the second Note kept none, and draws no box at all.
  const drawn = kept.getByRole("img", { name: /captured from/ });
  await expect.element(drawn).toBeVisible();
  await expect.poll(() => drawn.element().getAttribute("src")).toMatch(/^blob:/);
  expect(node(/^Note: It wraps at 720 wide/).getByRole("img").query()).toBeNull();

  // Opened from the node's own acts, and read-only is no reason not to look.
  kept.element().focus();
  await userEvent.keyboard("{Enter}");
  await act("Open frame");
  const sheet = page.getByRole("dialog", { name: "Note" });
  await expect.element(sheet).toBeVisible();
  await expect.element(sheet.getByText("The legend under the step bar is unreadable")).toBeVisible();
  await expect.element(sheet.getByRole("img", { name: /captured from/ })).toBeVisible();

  // A Note that kept none is offered nothing to open. Continued first, so what
  // the menu is missing is the frame rather than every act a read-only Studio
  // withholds.
  await userEvent.keyboard("{Escape}");
  await page.getByRole("button", { name: "Continue" }).click();
  await pick(/^Note: It wraps at 720 wide/);
  await openActs();
  await expect.element(offered("Write up")).toBeVisible();
  expect(offered("Open frame").query()).toBeNull();
});

test("two Notes clustered, the Cluster written up, the draft edited and dispatched to a Job node", async () => {
  const fleet = studying([]);
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();
  fleet.twoNotes(fleet.studios()[0]!.id);
  await expect.element(node(/^Note: The chip keeps its count/)).toBeVisible();

  // Picked together: the second joins the first rather than replacing it.
  await pick(/^Note: The chip keeps its count/);
  await userEvent.keyboard("{Meta>}");
  await node(/^Note: Overview still says three/).click();
  await userEvent.keyboard("{/Meta}");
  await openActs();
  await expect.element(offered("Cluster Notes")).toBeVisible();

  await act("Cluster Notes");
  await page.getByRole("textbox", { name: "Title" }).fill("Counts go stale");
  await asked("Cluster").click();
  await expect.element(node(/^Cluster: Counts go stale/)).toBeVisible();
  // Every Note it was made of keeps an edge to it, so the Cluster says where it came from.
  await expect.poll(() => fleet.studios()[0]!.edges.filter((edge) => edge.kind === "produced").length).toBe(2);

  await pick(/^Cluster: Counts go stale/);
  await act("Write up");
  // The write-up opens on the Notes' own words rather than on an empty field.
  await expect
    .poll(() => (page.getByRole("textbox", { name: "Body" }).element() as HTMLTextAreaElement).value)
    .toContain("The chip keeps its count");
  await page.getByRole("textbox", { name: "Title" }).fill("Counts go stale after what they count changes");
  await asked("Write up").click();
  await expect.element(node(/^Issue draft: Counts go stale after what they count changes/)).toBeVisible();

  // Edited before it is sent: what is dispatched is what the person left.
  await pick(/^Issue draft: Counts go stale after what they count changes/);
  await act("Edit draft");
  await page.getByRole("textbox", { name: "Body" }).fill("Both counts are read off a row that is stale.");
  await asked("Save draft").click();
  await expect
    .poll(() => fleet.studios()[0]!.nodes.find((one) => one.kind === "issue_draft"))
    .toMatchObject({ body: "Both counts are read off a row that is stale." });

  await pick(/^Issue draft: Counts go stale after what they count changes/);
  await act("Dispatch");
  // What is sent is the draft's own text, title first, and a person reads it before pressing.
  await expect
    .poll(() => (page.getByRole("textbox", { name: "What is sent" }).element() as HTMLTextAreaElement).value)
    .toBe("Counts go stale after what they count changes\n\nBoth counts are read off a row that is stale.");
  await asked("Dispatch").click();

  await expect.poll(() => fleet.studios()[0]!.nodes.filter((one) => one.kind === "job").length).toBe(1);
  const studio = fleet.studios()[0]!;
  const job = studio.nodes.find((one) => one.kind === "job")!;
  const draft = studio.nodes.find((one) => one.kind === "issue_draft")!;
  expect(studio.edges.find((edge) => edge.to === job.id)).toMatchObject({
    from: draft.id,
    kind: "produced",
  });
});

test("a Contradiction is ended as Resolved here, with the answer kept on the node", async () => {
  const fleet = studying([]);
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();
  fleet.aContradiction(fleet.studios()[0]!.id);
  await expect.element(node(/^Contradiction: The chip reads the Board/)).toBeVisible();

  await pick(/^Contradiction: The chip reads the Board/);
  // All four outcomes are offered on the node, and two of them are the rungs beside them.
  await openActs();
  for (const one of ["Write up", "Defer", "Not a problem", "Resolved here"]) {
    await expect.element(offered(one)).toBeVisible();
  }
  await offered("Resolved here").click();
  await page.getByRole("textbox", { name: "Answer" }).fill("The Board wins; the row is stale");
  await asked("Resolve").click();

  await expect
    .poll(() => fleet.studios()[0]!.nodes[0])
    .toMatchObject({
      state: "resolved_here",
      answer: "The Board wins; the row is stale",
    });
  // Ended once: nothing offers a second outcome on it.
  await pick(/^Contradiction: The chip reads the Board/);
  await openActs();
  expect(offered("Not a problem").query()).toBeNull();
});

/** The address the owner pasted, long enough that a card cannot hold it whole. */
const PASTED = "https://example.invalid/armada/issues/1378#issuecomment-2847190034-and-then-some";

/**
 * #1378's own definition of done: a person pastes an address, is asked what to do with it, types a
 * line saying why they kept it, and reads that line on the card with the address under it.
 *
 * **The reopen is half of it.** The line is a field on the Link's content rather than a second
 * node, so what proves it is not lost is the same close-and-reopen `#1287` uses — and editing it
 * afterwards has to reach the record too, not just the card.
 */
test("a pasted address is asked about, takes a line of its own, and keeps it across a reopen", async () => {
  const fleet = studying([]);
  const first = open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();

  await userEvent.keyboard("V");
  // Nothing is offered until there is an address to offer it about.
  expect(page.getByLabelText("Your line", { exact: true }).query()).toBeNull();
  await userEvent.fill(page.getByLabelText("Link", { exact: true }), PASTED);

  const offer = page.getByRole("group", { name: "What to do with this address" });
  await expect.element(offer).toBeVisible();
  // Reading in is #1293's, and this build says so rather than drawing the choice dead.
  await expect.element(offer.getByText("not built yet", { exact: false })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Read it in" })).toBeDisabled();

  await userEvent.fill(page.getByLabelText("Your line", { exact: true }), "the owner's own report");
  await page.getByRole("button", { name: "Keep the link" }).click();
  await expect.element(node(/^Link: the owner's own report/)).toBeVisible();
  // The card reads the line and the address is still under it, whole in its title.
  await expect.element(page.getByTitle(PASTED)).toBeVisible();

  close();
  open(fleet.scenario, first.api);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "Untitled Studio", exact: true }).click();
  await expect.element(node(/^Link: the owner's own report/)).toBeVisible();

  await page.getByRole("button", { name: "Continue" }).click();
  await pick(/^Link: the owner's own report/);
  await act("Edit line");
  await entered(page.getByRole("dialog"));
  await userEvent.fill(page.getByLabelText("Your line", { exact: true }), "why the card said nothing");
  await asked("Save line").click();
  await expect.element(node(/^Link: why the card said nothing/)).toBeVisible();
  await expect.poll(() => fleet.studios()[0]!.nodes[0]).toMatchObject({
    kind: "link",
    address: PASTED,
    said: "why the card said nothing",
  });
});

test("an issue is read in and an epic fills the board, with each address node left standing", async () => {
  const fleet = studying();
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "The Board's legend", exact: true }).click();
  // Reopened read-only, so nothing acts on it until a person continues it.
  await page.getByRole("button", { name: "Continue" }).click();
  await expect.element(node(/^Issue: Read a source a person already has/)).toBeVisible();

  // One issue: a Finding beside the Notes and the Contradiction its scout asked for.
  await pick(/^Issue: Read a source a person already has/);
  await act("Read in");
  await asked("Read in").click();
  await expect.element(node(/^Finding: Read in https:\/\/example\.invalid\/o\/r\/issues\/1293, frozen/)).toBeVisible();
  await expect.element(node(/^Note: The issue wants Links read in/)).toBeVisible();
  await expect.element(node(/^Contradiction: The issue says Connections have no home yet/)).toBeVisible();

  const studio = () => fleet.studios()[0]!;
  // The node keeps its address whatever came back, and everything hangs off it.
  expect(studio().nodes.find((one) => one.id === "legend-issue")).toMatchObject({
    kind: "issue",
    address: "https://example.invalid/o/r/issues/1293",
    number: "1293",
  });
  expect(studio().edges.filter((edge) => edge.from === "legend-issue" && edge.kind === "produced")).toHaveLength(4);
  // A relation the scout asked for waits on a person.
  expect(studio().edges.filter((edge) => edge.standing === "proposed" && edge.kind === "blocks")).toHaveLength(1);

  // An Epic: one Issue per issue, each carrying the filed issue's address, and
  // the Epic itself saying how many of how many were read in. **It asks what
  // to take first** — #1405 — and taking every issue is what fills the board.
  await pick(/^Epic: Studio/);
  await act("Read in");
  await userEvent.selectOptions(page.getByLabelText("Take", { exact: true }), "everything");
  await asked("Read in").click();
  await expect.element(node(/^Issue: An issue cannot be read into a Studio/)).toBeVisible();
  await expect.element(node(/^Issue: Kit manages connections/)).toBeVisible();

  const issues = studio()
    .edges.filter((edge) => edge.from === "legend-milestone" && edge.kind === "produced")
    .map((edge) => studio().nodes.find((one) => one.id === edge.to));
  expect(issues.map((one) => (one?.kind === "issue" ? one.address : null))).toEqual([
    "https://example.invalid/o/r/issues/1293",
    "https://example.invalid/o/r/issues/1291",
    "https://example.invalid/o/r/issues/1275",
  ]);
  // Its own address survives, and how much of it landed is two numbers.
  expect(studio().nodes.find((one) => one.id === "legend-milestone")).toMatchObject({
    address: "https://example.invalid/o/r/milestone/17",
    read_in: { issues: 3, total: 3, took: "everything", left_out: 0, kept: 0 },
  });
});

/**
 * `#1405`'s definition of done, on Bridge: **reading a milestone in asks
 * whether to take every issue or only the open ones, pressing the Epic
 * afterwards changes that answer, and a Note written against a closed issue
 * survives narrowing.**
 */
test("reading an epic in asks what to take, and narrowing leaves what a person worked on", async () => {
  const fleet = studying();
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "The Board's legend", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  const studio = () => fleet.studios()[0]!;
  const epic = () => studio().nodes.find((one) => one.id === "legend-milestone");

  // The offer asks, and opens on the answer a person planning work wants.
  await pick(/^Epic: Studio/);
  await act("Read in");
  await expect.element(page.getByRole("dialog", { name: "Read this epic in" })).toBeVisible();
  await expect.element(page.getByLabelText("Take", { exact: true })).toHaveValue("open");
  await asked("Read in").click();
  await expect.element(node(/^Issue: An issue cannot be read into a Studio/)).toBeVisible();
  await expect.poll(() => epic()).toMatchObject({ read_in: { issues: 2, total: 3, took: "open", left_out: 1 } });
  // The Epic says which state it took and how many it left out.
  await expect.element(node(/^Epic: Studio/)).toHaveTextContent("Open issues only");
  await expect.element(node(/^Epic: Studio/)).toHaveTextContent("1 left out");
  expect(studio().nodes.some((one) => one.kind === "issue" && one.number === "1291")).toBe(false);

  // Widening takes the closed one too, and a Note is written against it.
  await act("Read in");
  await userEvent.selectOptions(page.getByLabelText("Take", { exact: true }), "everything");
  await asked("Read in").click();
  await expect.element(node(/^Issue: Promotion: cluster, defer, write up/)).toBeVisible();
  await pick(/^Issue: Promotion: cluster, defer, write up/);
  await act("Defer");
  await userEvent.fill(page.getByLabelText("What is being put off", { exact: true }), "does this still hold?");
  await asked("Defer").click();
  await expect.element(node(/^Deferral: does this still hold\?/)).toBeVisible();

  // Narrowing again leaves it standing, and the Epic says it kept one.
  await pick(/^Epic: Studio/);
  await act("Read in");
  await expect.element(page.getByLabelText("Take", { exact: true })).toHaveValue("everything");
  await userEvent.selectOptions(page.getByLabelText("Take", { exact: true }), "open");
  await asked("Read in").click();
  await expect.poll(() => epic()).toMatchObject({ read_in: { issues: 3, took: "open", left_out: 1, kept: 1 } });
  await expect.element(node(/^Issue: Promotion: cluster, defer, write up/)).toBeVisible();
  await expect.element(node(/^Deferral: does this still hold\?/)).toBeVisible();
  await expect.element(node(/^Epic: Studio/)).toHaveTextContent("1 kept, worked on");
});

/**
 * `#1379`'s definition of done, which is the owner's own walkthrough: paste a
 * milestone's address, read it in, press Dispatch on one issue's node, open
 * that Job from the node and watch it run, then come back and see the same
 * node reading running.
 *
 * **The Job node holds no status.** What it draws here is the Board's row,
 * which is why the last assertion is worth making: the Studio was left
 * standing while the Job moved, and it says what the Job is doing rather than
 * what it was doing when the node was made.
 */
test("an issue read in is dispatched, its Job opens from the node, and the node reads it running", async () => {
  const fleet = studying();
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "The Board's legend", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();

  // The Epic fills the board with an Issue per issue — #1293.
  await pick(/^Epic: Studio/);
  await act("Read in");
  await asked("Read in").click();
  await expect.element(node(/^Issue: An issue cannot be read into a Studio/)).toBeVisible();

  // All three forge kinds dispatch, and the dialog names which it is about:
  // an issue, a pull request and an epic are three different asks — #1379,
  // #1394. Reading an Epic in and dispatching it are not rivals either.
  await pick(/^Pull request: where dispatch landed/);
  await act("Dispatch");
  await expect.element(page.getByRole("dialog", { name: "Dispatch this pull request" })).toBeVisible();
  await page.getByRole("dialog").getByRole("button", { name: "Cancel" }).click();

  await pick(/^Epic: Studio/);
  await act("Dispatch");
  await expect.element(page.getByRole("dialog", { name: "Dispatch this epic" })).toBeVisible();
  await page.getByRole("dialog").getByRole("button", { name: "Cancel" }).click();

  // A Link is an address no adapter recognised: nothing filed to dispatch.
  await pick(/^Link: why the ids collide/);
  await openActs();
  await expect.poll(() => offered("Dispatch").query()).toBeNull();

  await pick(/^Issue: An issue cannot be read into a Studio/);
  await act("Dispatch");
  // The address is the whole request. Nothing is filed — the issue already is.
  await expect
    .poll(() => (page.getByRole("textbox", { name: "What is sent" }).element() as HTMLTextAreaElement).value)
    .toBe("https://example.invalid/o/r/issues/1293");
  await asked("Dispatch").click();

  // The Job node lands with a `Produced` edge from the node it came from, and
  // stands at the gate like every other Job.
  const dispatched = /^Job: An issue cannot be read into a Studio/;
  await expect.element(node(dispatched)).toBeVisible();
  const studio = () => fleet.studios()[0]!;
  const jobNode = () => studio().nodes.find((one) => one.kind === "job")!;
  const edge = studio().edges.find((one) => one.to === jobNode().id)!;
  expect(edge.kind).toBe("produced");
  expect(studio().nodes.find((one) => one.id === edge.from)).toMatchObject({
    kind: "issue",
    address: "https://example.invalid/o/r/issues/1293",
  });

  // The node opens the Job, the way a Board row does.
  await pick(dispatched);
  await act("Open Job");
  // The gate is unchanged: a Job from a Studio stands at `awaiting_approval`
  // like any other, and this is where it is approved.
  await expect.element(page.getByRole("button", { name: "Approve dispatch" })).toBeVisible();
  await expect.element(page.getByText("Dispatched by you")).toBeVisible();

  await page.getByRole("button", { name: "Approve dispatch" }).click();
  await userEvent.keyboard("{Escape}");

  // Back on the Studio it was left on, and the node reads what the Job is
  // doing now rather than what it was doing when it was made.
  await expect.element(node(/^Job: An issue cannot be read into a Studio, running/)).toBeVisible();
});

/**
 * #1406's own: a person picks a node holding an address, presses Open, and it
 * opens in their browser with Bridge unchanged behind it.
 *
 * **What is proved is the address**, not the press: the renderer sends a Studio
 * and a node, and what reaches the browser is what the record carries.
 */
test("a node holding an address opens in the browser, and a node without one offers no Open", async () => {
  const fleet = studying();
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "The Board's legend", exact: true }).click();

  // Read-only is no reason not to look at what a node points at.
  await pick(/^Issue: Read a source a person already has/);
  await act("Open");
  await expect.poll(() => fleet.browsed()).toEqual(["https://example.invalid/o/r/issues/1293"]);

  await pick(/^Link: why the ids collide/);
  await act("Open");
  await expect.poll(() => fleet.browsed()).toEqual([
    "https://example.invalid/o/r/issues/1293",
    "https://react.dev/reference/react/useId",
  ]);

  await pick(/^Pull request: where dispatch landed/);
  await act("Open");
  await expect.poll(() => fleet.browsed()).toHaveLength(3);
  expect(fleet.browsed()[2]).toBe("https://example.invalid/o/r/pull/1391");

  await pick(/^Epic: Studio/);
  await act("Open");
  await expect.poll(() => fleet.browsed()).toHaveLength(4);
  expect(fleet.browsed()[3]).toBe("https://example.invalid/o/r/milestone/17");

  // A Finding holds no address, so nothing offers to open one — and Bridge is
  // still on the Studio, because no surface navigates.
  await page.getByRole("button", { name: "Continue" }).click();
  await pick(/^Finding: Where do the legend's colours come from/);
  await openActs();
  expect(offered("Open").query()).toBeNull();
  await expect.element(page.getByRole("heading", { name: "The Board's legend" })).toBeVisible();
});

/** Every node the board has drawn, in the order React Flow laid them out. */
const everyNode = () => Array.from(document.querySelectorAll<HTMLElement>(".react-flow__node"));

/**
 * Pick the whole board, the way a person does: hold the key that joins to the
 * selection and take each node in turn.
 *
 * **A click on the element, not a pointer at a point.** Eighteen nodes fitted to
 * the viewport sit under the board's own asides, and a pointer press lands on
 * whatever is drawn on top. **And a click rather than `Enter`**: React Flow
 * reads the held key through `useKeyPress`, which clears every key it is
 * holding on the *first* keyup it sees — so the second `Enter` would arrive
 * with the selection no longer joining and replace all of it.
 */
async function pickEvery(): Promise<void> {
  await userEvent.keyboard("{Meta>}");
  for (const one of everyNode()) one.click();
  await userEvent.keyboard("{/Meta}");
}

/**
 * #1411's own claim, and the owner's own case on 17 Sep: he picked every node
 * on a Studio to clear it out and there was no act for it.
 *
 * **One act, one confirmation, one write.** The act counts rather than naming —
 * eighteen titles is the panel that overflowed the window — and what goes with
 * them is said before the press: the edges on them, the picture a captured Note
 * keeps, and that a Job node's Job is left alone.
 */
test("every node picked is deleted by one act, confirmed once, and the Studio is left empty", async () => {
  const fleet = studying([everyKind("01JOBEVERYKIND0000000000000")]);
  open(fleet.scenario);
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "Every kind of node and edge", exact: true }).click();
  await expect.element(node(/^Note: The legend under the step bar is unreadable/)).toBeVisible();
  // Reopened read-only, so nothing is deleted until a person continues it.
  await page.getByRole("button", { name: "Continue" }).click();
  await expect.poll(() => everyNode().length).toBe(18);

  await pickEvery();
  await openActs();
  // Counted, never named, and the single-node act is not what is offered here.
  await expect.element(offered("Delete 18 nodes")).toBeVisible();

  await offered("Delete 18 nodes").click();
  const confirm = page.getByRole("dialog");
  await entered(confirm);
  await expect.element(page.getByRole("button", { name: "Cancel" })).toHaveFocus();
  // What goes with them, before the press.
  await expect.element(confirm.getByText("18 nodes go from this Studio, with the 15 edges on them.")).toBeVisible();
  await expect.element(confirm.getByText(/1 Note going keeps a picture/)).toBeVisible();
  await expect.element(confirm.getByText(/The Job it names is untouched/)).toBeVisible();
  await expect.element(confirm.getByText("There is no undo.")).toBeVisible();

  await confirm.getByRole("button", { name: "Delete 18 nodes" }).click();
  // One write: the Studio is empty, and every edge went with the nodes.
  await expect.poll(() => fleet.studios()[0]!.nodes).toEqual([]);
  expect(fleet.studios()[0]!.edges).toEqual([]);
  await expect.element(page.getByText("Nothing on this Studio yet.", { exact: false })).toBeVisible();
});
