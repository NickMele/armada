// What the capture window may load — #1294, `docs/practices/capture-window.md`.
//
// The subject is the three tests a window opens through: the link is one the
// live holder carries, the Run is still serving, and the address is a process
// on this machine.

import { describe, expect, it } from "vitest";

import type { ServerState } from "@armada/protocol";
import { NOTHING_YET, type BridgeState } from "../../shared/bridge";
import { isPinned, loopbackOrigin, offerable, onOrigin, partitionFor, pinned } from "./address";

const SERVING: ServerState = {
  id: "01SERVER",
  name: "web_dev",
  manifest_id: "01MANIFEST",
  phase: "serving",
  serve: "pnpm dev --port 41207",
  ports: [{ name: "web", port: 41207 }],
  links: [
    { url: "http://localhost:41207", name: "The app" },
    { url: "http://127.0.0.1:41207/admin" },
  ],
  started_by: "person",
  started_at: "2026-09-18T10:00:00Z",
  stopped: false,
  log: "servers/01SERVER.log",
};

const holding = (...servers: ServerState[]): BridgeState => ({
  ...NOTHING_YET,
  servers: { servers },
});

describe("what the capture window may load", () => {
  it("resolves one of the live holder's own links, and nothing a caller composed", () => {
    const answer = pinned(holding(SERVING), "01SERVER", "http://localhost:41207");
    expect(isPinned(answer)).toBe(true);
    expect(answer).toMatchObject({
      run: "01SERVER",
      name: "web_dev",
      origin: "http://localhost:41207",
      url: "http://localhost:41207",
      manifestId: "01MANIFEST",
    });
  });

  it("refuses an address that server does not publish, however plausible", () => {
    // The port is the Run's and the path is not: the test is on the link, whole.
    expect(pinned(holding(SERVING), "01SERVER", "http://localhost:41207/secret")).toEqual({
      ok: false,
      why: "no_address",
    });
  });

  it("refuses a server nothing is holding", () => {
    expect(pinned(holding(), "01SERVER", "http://localhost:41207")).toEqual({ ok: false, why: "no_address" });
  });

  it("refuses a Run that has stopped serving, because a port is not an identity", () => {
    const exited = { ...SERVING, phase: "exited", stopped: true };
    expect(pinned(holding(exited), "01SERVER", "http://localhost:41207")).toEqual({
      ok: false,
      why: "not_serving",
    });
  });

  it("names the address when a Manifest declares a server that is not on this machine", () => {
    const remote: ServerState = { ...SERVING, links: [{ url: "https://staging.example.com" }] };
    expect(pinned(holding(remote), "01SERVER", "https://staging.example.com")).toEqual({
      ok: false,
      why: "not_loopback",
      address: "https://staging.example.com",
    });
  });
});

describe("a loopback origin", () => {
  it.each([
    ["http://127.0.0.1:41207", "http://127.0.0.1:41207"],
    ["http://localhost:3000/some/path?q=1", "http://localhost:3000"],
    ["https://localhost:8443", "https://localhost:8443"],
    ["http://[::1]:5173", "http://[::1]:5173"],
  ])("takes %s", (address, origin) => {
    expect(loopbackOrigin(address)).toBe(origin);
  });

  it.each([
    // The host is matched whole: nothing here is a prefix or a suffix test.
    "http://127.0.0.1.example.com",
    "http://localhost.evil.test",
    "http://notlocalhost",
    "http://192.168.1.4:3000",
    "file:///tmp/index.html",
    "data:text/html,<b>hi</b>",
    "about:blank",
    "javascript:alert(1)",
    "not an address",
  ])("refuses %s", (address) => {
    expect(loopbackOrigin(address)).toBeNull();
  });
});

describe("staying on the origin", () => {
  const origin = "http://localhost:41207";

  it("takes a path, a query and a hash on the same origin", () => {
    expect(onOrigin(origin, "http://localhost:41207/checkout?step=2#pay")).toBe(true);
  });

  it.each([
    // Scheme, host and port together — a different one of the three is off it.
    ["a different port", "http://localhost:41208/"],
    ["a different scheme", "https://localhost:41207/"],
    ["a different host on the same port", "http://127.0.0.1:41207/"],
    ["a host this one is a prefix of", "http://localhost:41207.example.com/"],
    ["an auth provider", "https://accounts.google.com/o/oauth2/v2/auth"],
    ["a scheme with no origin", "about:blank"],
    ["something unparseable", "http://[bad"],
  ])("refuses %s", (_what, address) => {
    expect(onOrigin(origin, address)).toBe(false);
  });
});

describe("what a refusal offers", () => {
  it("offers the system browser the web and nothing else", () => {
    expect(offerable("https://accounts.google.com/x")).toBe(true);
    expect(offerable("http://example.test")).toBe(true);
    expect(offerable("mailto:someone@example.test")).toBe(false);
    expect(offerable("file:///etc/passwd")).toBe(false);
    expect(offerable("bank://transfer")).toBe(false);
  });
});

describe("the session partition", () => {
  it("is persistent and one per repository, so two repositories share nothing", () => {
    expect(partitionFor("01MANIFEST")).toBe("persist:armada-capture-01MANIFEST");
    expect(partitionFor("01MANIFEST")).not.toBe(partitionFor("01OTHER"));
    expect(partitionFor(null)).toBe("persist:armada-capture-unnamed");
  });
});
