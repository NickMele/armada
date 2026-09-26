// What a browser test needs to drive `App` on a scenario: mount it, take it
// down after, and reach a surface the way a person does — by the rail.

import { afterEach, expect } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp } from "./mount";
import type { Mounted } from "./mount";
import type { Scenario } from "./scenario";

let mounted: { app: Mounted; host: HTMLElement }[] = [];

/** Registers the teardown. Call once at the top of a test file. */
export function unmountAfterEach(): void {
  afterEach(() => {
    for (const one of mounted) {
      one.app.unmount();
      one.host.remove();
    }
    mounted = [];
  });
}

/** Mount `App` on a scenario, in a host the app's stylesheet sizes as its window. */
export function mount(scenario: string | Scenario): Mounted {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(scenario, host);
  mounted.push({ app, host });
  return app;
}

/** Every window mounted in this test is drawn, with its effects run. */
export async function onScreen(): Promise<void> {
  await Promise.all(mounted.map((one) => one.app.onScreen));
}

/**
 * Put Helm's dock up, and wait for it. **Bridge opens with it shut** since
 * #1583: the dock draws over the content rather than beside it, so leaving it
 * standing would cover whatever the window was opened to read. `⌘J` is the
 * way in that works at every width and from inside a field.
 *
 * **The window has to be up before the press, not after it.** `⌘J` is bound in
 * an effect, and `mount` returns before React has run one — a keystroke aimed
 * at a window that is not listening yet reaches `window`, matches nothing, and
 * is gone, with no locator to have waited on and nothing to retry it. Measured
 * 23 Sep 2026 on #1592, once in 25 full runs under load.
 */
export async function openHelm(): Promise<void> {
  await onScreen();
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.element(page.getByRole("complementary", { name: "Helm" })).toBeVisible();
}

/**
 * Two windows on one main, side by side, each labelled as a region so a test
 * can scope to it. Both talk to one fake, so a clone either sends lands in both.
 */
export function mountTwo(scenario: Scenario, labels: [string, string]): [HTMLElement, HTMLElement] {
  const row = document.createElement("div");
  row.style.display = "flex";
  document.body.append(row);
  let api: Mounted["api"] | undefined;
  const hosts = labels.map((label) => {
    const host = document.createElement("section");
    host.setAttribute("aria-label", label);
    host.style.flex = "1";
    host.style.minWidth = "0";
    row.append(host);
    const app = mountApp(scenario, host, api);
    api = app.api;
    mounted.push({ app, host });
    return host;
  });
  // The row is not a window of its own: both of its windows are already tracked above.
  mounted.push({
    app: { api: api!, scenario, onScreen: Promise.resolve(), unmount: () => undefined },
    host: row,
  });
  return hosts as [HTMLElement, HTMLElement];
}

/**
 * A sheet or dialog that has finished entering: travelled in, or scaled up, by its own animation.
 * Visible holds from its first frame, while a sheet can still sit wholly past the window's
 * trailing edge with its travel not yet started: under a loaded full run a press aimed at it then
 * reached nothing, and the test read on as if it had landed — #1252. Its own animations only,
 * never its subtree's, where a running step's pulse never finishes. One that does not animate
 * returns at once.
 */
export async function entered(layer: ReturnType<typeof page.getByRole>): Promise<void> {
  await expect.element(layer).toBeVisible();
  await Promise.all(layer.element().getAnimations().map((one) => one.finished));
}

/** Every Board row drawn, in either arrangement. */
export const rows = (): HTMLElement[] => [...document.querySelectorAll<HTMLElement>("[data-job-id]")];

/** The Board, by its rail item, once it has drawn a row. `App` opens on Overview. */
export async function openBoard(): Promise<void> {
  await page.getByRole("button", { name: "Job Board" }).first().click();
  await expect.poll(() => rows().length).toBeGreaterThan(0);
}
