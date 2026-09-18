import { expect, test } from "vitest";
import type { CheckoutRunSheet, CheckoutRunSheetRead } from "@armada/protocol";

import { startId, studioStartEntries, studioStarts } from "./studio-starting";

const entry = (name: string) => ({
  name,
  run: `run ${name}`,
  requires: [],
  destructive: false,
  narrows: false,
  expect_exit_code: 0,
  frozen: false,
});

const SHEET: CheckoutRunSheet = {
  setup: [entry("bootstrap")],
  checks: [entry("typecheck")],
  commands: [entry("fmt")],
  servers: [{ name: "storybook_dev", serve: "pnpm storybook dev", links: [], destructive: false }],
};

const read: CheckoutRunSheetRead = { state: "read", sheet: SHEET };

test("what a Studio can start is the checkout's Checks, Commands and servers, and not its setup", () => {
  expect(studioStarts(read)).toEqual([
    { id: startId("typecheck", false), name: "typecheck", server: false },
    { id: startId("fmt", false), name: "fmt", server: false },
    { id: startId("storybook_dev", true), name: "storybook_dev", server: true },
  ]);
});

test("nothing read is nothing startable, which is what draws no control at all", () => {
  expect(studioStarts({ state: "none" })).toEqual([]);
  expect(studioStartEntries([])).toBeUndefined();
});

test("a server's id is not a run's, so one name declared as both stays two entries", () => {
  // The two ids differ by which operation the press reaches, which is the whole
  // of the difference on this side: a server is held, a run ends.
  expect(startId("dev", true)).not.toBe(startId("dev", false));
});

test("the menu groups them, servers under their own label", () => {
  expect(studioStartEntries(studioStarts(read))).toEqual([
    { kind: "label", id: "runs", label: "Checks and Commands" },
    { kind: "item", id: startId("typecheck", false), label: "typecheck" },
    { kind: "item", id: startId("fmt", false), label: "fmt" },
    { kind: "separator", id: "before-servers" },
    { kind: "label", id: "servers", label: "Servers" },
    { kind: "item", id: startId("storybook_dev", true), label: "storybook_dev" },
  ]);
});

test("a repository with nothing to serve draws no Servers group and no separator", () => {
  const nothing: CheckoutRunSheetRead = { state: "read", sheet: { ...SHEET, servers: undefined } };
  expect(studioStartEntries(studioStarts(nothing))).toEqual([
    { kind: "label", id: "runs", label: "Checks and Commands" },
    { kind: "item", id: startId("typecheck", false), label: "typecheck" },
    { kind: "item", id: startId("fmt", false), label: "fmt" },
  ]);
});
