// Studio capture, through `App`, on a Fleet that keeps Studios — #1290's
// definition of done: open a Studio, press the binding, point at something on
// another surface, say what is wrong, and find it on the Studio as a Note.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp, type Mounted } from "./mount";
import { studying } from "./studio-fleet";

let open: { app: Mounted; host: HTMLElement } | null = null;

function window_(scenario: Parameters<typeof mountApp>[0]): void {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  open = { app: mountApp(scenario, host), host };
}

afterEach(() => {
  open?.app.unmount();
  open?.host.remove();
  open = null;
});

/** ⌥⌘C, the registry's binding for capture. */
async function binding(): Promise<void> {
  await userEvent.keyboard("{Alt>}{Meta>}c{/Meta}{/Alt}");
}

const bar = () => page.getByRole("status").filter({ hasText: "Capturing" });

test("a Studio open, a press on another surface, and the note lands on it", async () => {
  const fleet = studying([]);
  window_(fleet.scenario);

  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "New Studio" }).click();
  await expect.element(page.getByRole("heading", { name: "Untitled Studio" })).toBeVisible();

  // Away to the Board: what is wrong is on the surface a person is looking at,
  // and the aim is the Studio they left open.
  await page.getByRole("button", { name: "Job Board", exact: true }).first().click();
  await binding();
  await expect.element(bar()).toHaveTextContent("Onto Untitled Studio");

  const pointed = page.getByRole("button", { name: "Studios", exact: true }).first();
  await pointed.click();
  const card = page.getByRole("dialog", { name: "Capture a note" });
  await expect.element(card).toBeVisible();
  // The press was swallowed: capture points, and the rail did not move.
  expect(page.getByRole("heading", { name: "Studios" }).query()).toBeNull();

  await userEvent.fill(card.getByLabelText("Note"), "The rail's count is stale after a kill");
  await card.getByRole("button", { name: "Capture" }).click();
  // Capturing ends with the note, so the app is usable again without a second press.
  await expect.poll(() => bar().query()).toBeNull();

  const [studio] = fleet.studios();
  const note = studio!.nodes.find((one) => one.kind === "note");
  expect(note).toBeDefined();
  if (note?.kind !== "note") throw new Error("a Note");
  expect(note.said).toBe("The rail's count is stale after a kill");
  expect(note.added_by).toBe("person");
  // What it points at: the selector finds the element again, and the styles and
  // the markup are what the layer does not record and a Note does.
  const capture = note.capture;
  expect(capture?.selector).toMatch(/button/);
  expect(capture?.markup).toMatch(/Studios/);
  expect(capture?.styles?.["font-size"]).toBeDefined();
  expect(capture?.screen).toBe("Job Board");
  expect(capture?.source).toBeUndefined();

  // And on the Studio, reopened from the list.
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "Untitled Studio" }).click();
  await expect.element(page.getByRole("group", { name: /^Note: The rail's count is stale/ })).toBeVisible();
});

test("with no Studio open the bar says so, and the card offers no Capture", async () => {
  window_(studying([]).scenario);
  await expect.element(page.getByRole("button", { name: "Job Board", exact: true }).first()).toBeVisible();

  await binding();
  await expect.element(bar()).toHaveTextContent("No Studio open");

  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  const card = page.getByRole("dialog", { name: "Capture a note" });
  await userEvent.fill(card.getByLabelText("Note"), "nowhere to land");
  await expect.element(card.getByRole("button", { name: "Capture" })).toBeDisabled();
});
