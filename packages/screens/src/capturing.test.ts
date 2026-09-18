// What a person is told when the capture window did not open — #1294.

import { describe, expect, it, vi } from "vitest";

import { captureOn, whyNotCaptured } from "./capturing";

describe("why the capture window did not open", () => {
  it("says nothing on success, because the window is in front of the person", () => {
    expect(whyNotCaptured({ ok: true })).toBeNull();
  });

  it("sends a stopped Run back to starting the server", () => {
    expect(whyNotCaptured({ ok: false, why: "not_serving" })).toContain("Start the server again");
  });

  it("sends a stale reading back to the Studio", () => {
    expect(whyNotCaptured({ ok: false, why: "no_address" })).toContain("Reopen the Studio");
  });

  it("names the address a Manifest declared that is not on this machine", () => {
    const said = whyNotCaptured({ ok: false, why: "not_loopback", address: "https://staging.example.test" });
    expect(said).toContain("https://staging.example.test");
    expect(said).toContain("browser");
  });

  it("says a Note would have nowhere to land when no Studio holds the Run", () => {
    expect(whyNotCaptured({ ok: false, why: "no_studio" })).toContain("nowhere to land");
  });
});

describe("asking for the window", () => {
  it("hands over the server id and its own link, and answers with the sentence", async () => {
    const open = vi.fn().mockResolvedValue({ ok: false, why: "not_serving" } as const);
    const said = await captureOn(open, "01SERVER", "http://localhost:41207");
    expect(open).toHaveBeenCalledWith("01SERVER", "http://localhost:41207");
    expect(said).toContain("Start the server again");
  });
});
