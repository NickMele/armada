// The web app, started from a Studio — #1345's definition of done, through
// `App`: the node reads serving with its link and Stop, the link goes to the
// system browser, and stopping it from the node ends it.
//
// **The node reads the live holder.** What the Studio keeps is the instance's
// id; everything drawn on the card comes from the server list this window is
// already holding, so the fixture publishes a Studio and a server rather than a
// node carrying a copy of a state.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { ServerState, Studio, StudioNode } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { sheet } from "./manifest-fleet";
import { mountApp, type Mounted } from "./mount";
import { studying } from "./studio-fleet";

const windows: { app: Mounted; host: HTMLElement }[] = [];

function open(scenario: Parameters<typeof mountApp>[0]): Mounted {
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(scenario, host);
  windows.push({ app, host });
  return app;
}

afterEach(() => {
  for (const one of windows.splice(0)) {
    one.app.unmount();
    one.host.remove();
  }
});

const node = (name: RegExp) => page.getByRole("group", { name });
const offers = () => page.getByRole("button", { name: "Acts", exact: true });
async function act(name: string): Promise<void> {
  await expect.element(offers()).toBeVisible();
  if (offers().element().getAttribute("aria-expanded") !== "true") await offers().click();
  await page.getByRole("menuitem", { name, exact: true }).click();
}

/** The instance Fleet is holding, as `list_servers` answers it. */
const SERVING: ServerState = {
  id: "01SERVERSTORYBOOK000000000",
  name: "storybook_dev",
  phase: "serving",
  serve: "pnpm -C packages/components exec storybook dev -p 41207 --no-open --ci",
  ports: [{ name: "storybook", port: 41207 }],
  links: [{ url: "http://localhost:41207", name: "Storybook" }],
  started_by: "person",
  started_at: "2026-09-17T08:30:00Z",
  serving_since: "2026-09-17T08:30:12Z",
  stopped: false,
  log: ".armada/servers/main/01SERVERSTORYBOOK000000000/output.log",
};

const AT = "2026-09-17T08:30:00Z";

/** A Studio holding one Run node, and the node holds the instance above. */
function servingStudio(): Studio {
  const held: StudioNode = {
    id: "studio-server",
    kind: "run",
    run_id: SERVING.id,
    held: "server",
    position: { x: 0, y: 0 },
    created_at: AT,
  };
  return {
    id: "01STUDIOSERVING0000000000",
    manifest_id: repository().manifest!.id,
    name: "The app, running",
    created_at: AT,
    touched_at: AT,
    nodes: [held],
    edges: [],
  };
}

/** `studying`, with the server Fleet holds and the run sheet the checkout declares. */
function serving() {
  const fleet = studying([servingStudio()]);
  return {
    ...fleet,
    scenario: {
      ...fleet.scenario,
      state: { ...fleet.scenario.state, servers: { servers: [SERVING] } },
      behaves: (handle: Parameters<NonNullable<typeof fleet.scenario.behaves>>[0]) => ({
        ...fleet.scenario.behaves?.(handle),
        watchCheckoutRunSheet: async (want: boolean) =>
          handle.publish({
            checkoutRunSheet: want ? { state: "read" as const, sheet: sheet() } : { state: "none" as const },
          }),
      }),
    },
  };
}

async function openTheStudio(): Promise<void> {
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "The app, running", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
}

/**
 * #1345's definition of done, in one pass. **The card says serving and how long
 * it has been up**; its link opens in the system browser rather than anywhere
 * in Bridge; and Stop on the node ends the instance Fleet holds.
 */
test("a server node reads serving with its link and Stop, and the link leaves Bridge", async () => {
  const fleet = serving();
  const app = open(fleet.scenario);
  await openTheStudio();

  // The name is the server's, off the live holder — the node keeps only its id.
  await expect.element(node(/^Run: storybook_dev, serving/)).toBeVisible();
  await expect.element(page.getByText("localhost:41207")).toBeVisible();

  node(/^Run: storybook_dev/).element().focus();
  await userEvent.keyboard("{Enter}");

  const openServerLink = vi.spyOn(app.api, "openServerLink");
  await act("Open Storybook");
  await expect.poll(() => openServerLink.mock.calls.length).toBe(1);
  expect(openServerLink).toHaveBeenCalledWith(SERVING.id, SERVING.links[0]!.url);

  const stopServer = vi.spyOn(app.api, "stopServer");
  await act("Stop the server");
  await expect.poll(() => stopServer.mock.calls.length).toBe(1);
  expect(stopServer).toHaveBeenCalledWith(SERVING.id);
});

/**
 * **The web app is startable from a Studio at all**, which is what #1345 is
 * named for: the control lists what the checkout declares, and a server goes to
 * its own operation because `start_run` refuses a name carrying `serve`.
 */
test("the Run control starts a declared server through start_studio_server", async () => {
  const fleet = serving();
  const app = open(fleet.scenario);
  await openTheStudio();

  const startStudioServer = vi.spyOn(app.api, "startStudioServer");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await page.getByRole("menuitem", { name: "storybook_dev", exact: true }).click();
  await expect.poll(() => startStudioServer.mock.calls.length).toBe(1);
  expect(startStudioServer.mock.calls[0]![1]).toBe("storybook_dev");

  // And a Check goes to the other one, which is the whole of the difference.
  const startStudioRun = vi.spyOn(app.api, "startStudioRun");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await page.getByRole("menuitem", { name: "typecheck", exact: true }).click();
  await expect.poll(() => startStudioRun.mock.calls.length).toBe(1);
  expect(startStudioRun.mock.calls[0]![1]).toBe("typecheck");
});
