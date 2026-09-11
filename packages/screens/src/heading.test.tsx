// `Unrenderable`, the header's dead end — and what it names when it draws.
//
// **The badge is the header, so there is no partial render to fall back to.**
// What this file pins is that the sentence names which registry is actually
// short: `JOB_STATUS` has no row at all, or `JOB_STATUS` has one and
// `JOB_LIFECYCLE` does not. It used to say "variant" in both cases, including
// once for a status this build could fully describe — `escalated` with no
// reason, which `render.ts` no longer routes here at all. See `render.test.ts`
// for that half.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import type { JobSummary } from "@armada/protocol";

import { Unrenderable } from "./heading";
import { mount, unmount } from "./mounted";

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    ...over,
  };
}

afterEach(() => {
  unmount();
});

test("names the variant where the wire spelling has no row at all", async () => {
  mount(<Unrenderable job={job({ status: "translated_into_greek" })} />);
  await expect
    .element(page.getByText("The registry carries no variant for it, so this Job has no detail to draw."))
    .toBeInTheDocument();
});
