// Opening one Job and then another, in the same panel.
//
// **The defect this exists for.** `JobDetail` stays mounted when the open Job
// changes, so every piece of reading state it holds is the previous Job's until
// something resets it. Five pieces were reset by effects — a frame late — and
// the rest were never reset at all: which steps' facts were open in the run
// tree was carried from one Job into the next, so a person who closed a step
// on one Job opened the next with that step still closed.
//
// **Two Jobs from one fixture**, stamped with different ids. Every read the
// panel draws carries the id it was taken for, so both have to move together or
// the panel draws neither.
import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { Watched } from "@armada/protocol";

import { JobDetail } from "./JobDetail";
import type { JobFixture } from "./fixtures/fixture";
import { running } from "./fixtures/build/running";
import { propsFor } from "./fixtures/props";
import { mount, rerender, unmount } from "./mounted";

afterEach(unmount);

/** A read, stamped for another Job where it names one. */
function stamped<T>(read: T, id: string): T {
  return typeof read === "object" && read !== null && "jobId" in read ? { ...read, jobId: id } : read;
}

/** The same moment, as another Job. */
function asAnotherJob(fixture: JobFixture, id: string): JobFixture {
  const watched: Watched =
    fixture.watched.state === "read"
      ? {
          ...fixture.watched,
          jobId: id,
          detail: { ...fixture.watched.detail, job: { ...fixture.watched.detail.job, id } },
        }
      : fixture.watched;
  return {
    ...fixture,
    job: { ...fixture.job, id },
    watched,
    observed: stamped(fixture.observed, id),
    journalled: stamped(fixture.journalled, id),
    resources: stamped(fixture.resources, id),
    ...(fixture.history === undefined ? {} : { history: stamped(fixture.history, id) }),
    recorded: {
      footprint: stamped(fixture.recorded.footprint, id),
      evidence: stamped(fixture.recorded.evidence, id),
      diff: stamped(fixture.recorded.diff, id),
      remarks: stamped(fixture.recorded.remarks, id),
    },
  };
}

test("the next Job opens with its own step's facts open, whatever the last one closed", async () => {
  const first = running();
  mount(<JobDetail {...propsFor(first)} />);

  const close = page.getByRole("button", { name: "Close this step's facts" });
  await expect.element(close).toBeVisible();
  await userEvent.click(close);
  await expect.element(page.getByRole("button", { name: "Open this step's facts" }).first()).toBeVisible();

  rerender(<JobDetail {...propsFor(asAnotherJob(first, "01M2NEXTJOB00000000000000A"))} />);

  // A fresh Job seeds its tree from its own current step, which starts open.
  await expect.element(page.getByRole("button", { name: "Close this step's facts" })).toBeVisible();
});
