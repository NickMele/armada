// Nobody looked, versus a comparison that found nobody.

import type { ScopeOverlap } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { peerOverlapOf } from "./peers";
import { sampleDetail } from "./sample";

const overlap: ScopeOverlap = {
  job_id: "01OTHER",
  title: "Restructure the five-tab shell",
  status: "running",
  paths: [{ path: "packages/screens/src/JobDetail.tsx" }],
};

describe("the difference between absent and empty", () => {
  it("answers null where nothing was compared", () => {
    expect(peerOverlapOf(sampleDetail())).toBeNull();
  });

  it("answers an empty peer list where a comparison found nobody", () => {
    const view = peerOverlapOf(sampleDetail({ write_scope_overlaps: [] }));

    expect(view).not.toBeNull();
    expect(view?.peers).toEqual([]);
  });

  it("keeps them apart, which is the whole point of the field", () => {
    expect(peerOverlapOf(sampleDetail())).not.toEqual(
      peerOverlapOf(sampleDetail({ write_scope_overlaps: [] })),
    );
  });
});

describe("what a peer row carries", () => {
  it("is one entry per Job, with its shared paths beneath it", () => {
    const view = peerOverlapOf(
      sampleDetail({
        write_targets: ["packages/screens/src"],
        write_scope_overlaps: [overlap],
      }),
    );

    expect(view?.peers).toEqual([
      {
        job: "01OTHER",
        title: "Restructure the five-tab shell",
        status: "running",
        shared_paths: ["packages/screens/src/JobDetail.tsx"],
      },
    ]);
    expect(view?.paths_asked).toEqual(["packages/screens/src"]);
  });

  it("carries no verdict a control could be greyed from", () => {
    const view = peerOverlapOf(sampleDetail({ write_scope_overlaps: [overlap] }));
    const peer = view?.peers[0];

    expect(peer && "blocked" in peer).toBe(false);
    expect(peer && "severity" in peer).toBe(false);
  });

  it("asks against no paths where the Job has claimed none", () => {
    expect(peerOverlapOf(sampleDetail({ write_scope_overlaps: [overlap] }))?.paths_asked).toEqual(
      [],
    );
  });
});
