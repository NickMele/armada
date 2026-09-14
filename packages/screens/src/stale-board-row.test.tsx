// The badge reads the Job's own fetched answer, never a board row that lagged.
//
// **The defect this exists for.** The Board's row for an open Job is patched
// piecemeal off `job.state_changed` as Bridge's main process hears them; the
// panel also fetches `GET /jobs/:job_id` for the Job that is actually open,
// which `run.ts` already trusts for step state. `JobDetail.tsx` used to read
// the board row for the header, the acts, the settings and the facts anyway —
// so a Job seen running, with its board row still `queued` because a
// `job.state_changed` had not yet landed or applied, drew "Queued" under a run
// tree that correctly said "step running". Reproduced on a real Job (#18).
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { JobDetail } from "./JobDetail";
import { running } from "./fixtures/build/running";
import { propsFor } from "./fixtures/props";
import { mount, unmount } from "./mounted";

afterEach(unmount);

test("the header badge takes the fetched detail's status over a board row that has fallen behind", async () => {
  const fixture = running();
  // The board row Bridge still holds — `queued` — while the fetched detail
  // this same fixture carries already reads `running`, the way a missed or
  // not-yet-applied `job.state_changed` leaves the two disagreeing.
  const stale = { ...fixture, job: { ...fixture.job, status: "queued" } };

  mount(<JobDetail {...propsFor(stale)} />);

  await expect.element(page.getByText(fixture.job.title).first()).toBeVisible();
  const badge = document.querySelector(".armada-badge");
  expect(badge?.getAttribute("data-status")).toBe("running");
  // The word a person actually reads, not only the token the CSS keys on —
  // "Queued" under a run tree already reading "running" is the defect #18
  // reported, and a fix that only moved the token could still leave it.
  expect(badge?.textContent).toContain("Running");
  expect(badge?.textContent).not.toContain("Queued");
});
