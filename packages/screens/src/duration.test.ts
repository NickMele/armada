// What the duration formatters say, at the boundaries where they change words.
import { describe, expect, it } from "vitest";

import { briefly, lasting, span } from "./duration";

describe("a short duration keeps its milliseconds", () => {
  it("reports under a second in milliseconds", () => {
    expect(briefly(200)).toBe("200ms");
  });

  it("hands a second and over to the same formatter every other span uses", () => {
    expect(briefly(1_000)).toBe("1s");
    expect(briefly(142_000)).toBe(lasting(142_000));
  });

  it("never reports a negative duration", () => {
    expect(briefly(-5)).toBe("0ms");
  });

  it("is why the body does not use `lasting` for a call", () => {
    // 343 of the recorded step's 351 calls answered inside a second, and
    // `lasting` rounds every one of them to the same word.
    expect(lasting(200)).toBe("0s");
    expect(briefly(200)).not.toBe(lasting(200));
  });
});

describe("a span between two instants", () => {
  it("says nothing where an end will not parse", () => {
    expect(span("2026-09-10T14:00:00Z", "not a date")).toBeNull();
  });

  it("reads minutes and seconds", () => {
    expect(span("2026-09-10T14:00:00Z", "2026-09-10T14:02:22Z")).toBe("2m 22s");
  });
});
