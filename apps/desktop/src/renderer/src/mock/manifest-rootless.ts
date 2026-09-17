// A repository with no root `armada.yml`, whose `apps/web` declares its own `dev`.

import type { CheckoutRunRecord, CheckoutRunSheet } from "@armada/protocol";

/** What `get_checkout_run_sheet` answers there. */
export function rootlessSheet(): CheckoutRunSheet {
  const dev = {
    name: "dev",
    run: "pnpm dev --port 5173",
    narrows: false,
    requires: [],
    expect_exit_code: 0,
    destructive: false,
    frozen: false,
  };
  return { setup: [], checks: [], commands: [], servers: [], workspaces: [{ dir: "apps/web", commands: [dev] }] };
}

/** `dev` run once in `apps/web`, as `list_checkout_runs` answers it. */
export const WEB_DEV_RUN: CheckoutRunRecord = {
  id: "01K4WEBDEV",
  name: "dev",
  workspace: "apps/web",
  command: "pnpm dev --port 5173",
  required: [],
  started_at: "2026-09-13T09:02:00Z",
  ended_at: "2026-09-13T09:02:41Z",
  duration_ms: 41200,
  exit_code: 130,
  expect_exit_code: 0,
  ended: "stopped",
  stopped: true,
  changed: [],
  undoable: false,
  log: "runs/main/01K4WEBDEV/output.log",
};
