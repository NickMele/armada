// What a browser test needs to drive `App` on a scenario: mount it, take it
// down after, and reach a surface the way a person does — by the rail.

import { afterEach, expect } from "vitest";
import { page } from "vitest/browser";

import { mountApp } from "./mount";
import type { Mounted } from "./mount";
import type { Scenario } from "./scenario";

let mounted: { app: Mounted; host: HTMLElement } | null = null;

/** Registers the teardown. Call once at the top of a test file. */
export function unmountAfterEach(): void {
  afterEach(() => {
    mounted?.app.unmount();
    mounted?.host.remove();
    mounted = null;
  });
}

/** Mount `App` on a scenario, in a host the app's stylesheet sizes as its window. */
export function mount(scenario: string | Scenario): Mounted {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(scenario, host);
  mounted = { app, host };
  return app;
}

/** Every Board row drawn, in either arrangement. */
export const rows = (): HTMLElement[] => [...document.querySelectorAll<HTMLElement>("[data-job-id]")];

/** The Board, by its rail item, once it has drawn a row. `App` opens on Overview. */
export async function openBoard(): Promise<void> {
  await page.getByRole("button", { name: "Job Board" }).first().click();
  await expect.poll(() => rows().length).toBeGreaterThan(0);
}
