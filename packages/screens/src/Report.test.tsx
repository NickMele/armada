// A report will not be filed without a sentence — by the button and by the key.
//
// # This one guards two things, not one
//
// `Report.tsx` refuses a blank sentence for the reason the other two do, and it
// also refuses a second press while a filing is in flight: `confirmDisabled` is
// `said.trim() === "" || filing`. The second half has no visible difference
// from the first, so a person cannot tell which refusal they are looking at —
// and a double press that got through would file the same report twice against
// one Job.
//
// # `whole: null` is a real state, not a stub
//
// The prop is `JobDetail | null` and the file documents it as the read that
// fills in the verdicts a criterion scope picks between. A dialog opened before
// that read lands is the state under test here, and it is the one the rule has
// to hold in — nothing about a blank sentence depends on the read.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Outcome } from "@armada/protocol";

import { mount, unmount } from "./mounted";
import { ReportControl } from "./Report";

afterEach(unmount);

const FIELD = "What you know went wrong";
const FILE_IT = "File the report";

/**
 * Mount the dialog open, and hand back what filing was told.
 *
 * `answers` is what `onReport` resolves with, and it is a promise the test
 * holds: nothing resolves until the test says so, which is how the in-flight
 * refusal below is reached at all.
 */
function opened(answers?: Promise<Outcome>): { filings: string[] } {
  const filings: string[] = [];
  mount(
    <ReportControl
      jobId="job_2d90bb"
      whole={null}
      open
      onClose={() => {}}
      onReport={(_jobId, filing) => {
        filings.push(filing.said);
        return answers ?? Promise.resolve({ ok: true });
      }}
      onCopied={() => {}}
    />,
  );
  return { filings };
}

function fileIt() {
  return page.getByRole("dialog").getByRole("button", { name: FILE_IT });
}

test("a blank sentence is refused by the button and by Enter", async () => {
  const { filings } = opened();

  await expect.element(fileIt()).toBeDisabled();

  await userEvent.keyboard("{Enter}");
  expect(filings, "Enter filed a report with no sentence").toEqual([]);
  await expect.element(page.getByRole("dialog")).toBeVisible();
});

test("whitespace is not a sentence", async () => {
  const { filings } = opened();

  await userEvent.fill(page.getByRole("textbox", { name: FIELD }), " \n\t ");

  await expect.element(fileIt()).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  expect(filings, "spaces counted as a sentence").toEqual([]);
});

test("a sentence satisfies it, and Enter then files", async () => {
  const { filings } = opened();

  await userEvent.fill(page.getByRole("textbox", { name: FIELD }), "the criterion asks for a test");
  await expect.element(fileIt()).toBeEnabled();

  await userEvent.keyboard("{Enter}");
  expect(filings).toEqual(["the criterion asks for a test"]);
});

test("a filing in flight refuses a second Enter", async () => {
  // Held open deliberately: the refusal under test only exists between the
  // press and the answer, and a resolved promise closes that window before a
  // second press can reach it.
  let land: (outcome: Outcome) => void = () => {};
  const answers = new Promise<Outcome>((resolve) => {
    land = resolve;
  });
  const { filings } = opened(answers);

  await userEvent.fill(page.getByRole("textbox", { name: FIELD }), "the criterion asks for a test");
  await userEvent.keyboard("{Enter}");
  expect(filings).toHaveLength(1);

  await expect.element(fileIt()).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  await userEvent.keyboard("{Enter}");
  expect(filings, "one Job was reported more than once").toHaveLength(1);

  land({ ok: true });
});
