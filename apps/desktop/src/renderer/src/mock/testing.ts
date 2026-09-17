// What a browser test needs to drive `App` on a scenario: mount it, take it
// down after, and reach a surface the way a person does — by the rail.

import { afterEach, expect } from "vitest";
import { page } from "vitest/browser";

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
  mounted.push({ app: { api: api!, scenario, unmount: () => undefined }, host: row });
  return hosts as [HTMLElement, HTMLElement];
}

/** Every Board row drawn, in either arrangement. */
export const rows = (): HTMLElement[] => [...document.querySelectorAll<HTMLElement>("[data-job-id]")];

/** The Board, by its rail item, once it has drawn a row. `App` opens on Overview. */
export async function openBoard(): Promise<void> {
  await page.getByRole("button", { name: "Job Board" }).first().click();
  await expect.poll(() => rows().length).toBeGreaterThan(0);
}
