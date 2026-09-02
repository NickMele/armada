// A redirect will not send a blank instruction — by the button and by the key.
//
// # Why this is a browser test and not a story
//
// `RedirectControl` holds the instruction and hands `Dialog` the answer to *is
// the field blank*. `packages/components` sits below this package and cannot
// import it, so no story can mount this control — a story there proves what
// `Dialog` does with `confirmDisabled`, and this proves this screen sets it.
//
// # The keyboard is a second refusal, not the same one
//
// `Dialog` refuses twice and independently: the confirm button carries
// `disabled`, and the `Enter` handler bound on `window` has its own
// `if (!confirmDisabled)`. A regression in the second leaves a dialog whose
// send control reads as refused and sends anyway from the keyboard, which is
// the shape a person reading the screen cannot see.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { ACT_LABEL } from "./copy";
import { mount, unmount } from "./mounted";
import { RedirectControl } from "./Redirect";

afterEach(unmount);

/** Open the dialog and hand back what the send call was told, as it is told. */
function opened(): { sent: [string, string][] } {
  const sent: [string, string][] = [];
  mount(
    <RedirectControl
      jobId="job_2d90bb"
      disabled={false}
      onRedirect={(jobId, instruction) => sent.push([jobId, instruction])}
    />,
  );
  return { sent };
}

/**
 * The send control, scoped to the dialog.
 *
 * **The opener and the confirm carry the same words on purpose** — the design
 * contract says an action keeps its name through the flow — so a query by name
 * alone finds two buttons and the one outside the dialog is never disabled.
 */
function send() {
  return page.getByRole("dialog").getByRole("button", { name: ACT_LABEL.redirect });
}

test("a blank instruction is refused by the button and by Enter", async () => {
  const { sent } = opened();
  await userEvent.click(page.getByRole("button", { name: ACT_LABEL.redirect }));

  await expect.element(send()).toBeDisabled();

  await userEvent.keyboard("{Enter}");
  expect(sent, "Enter sent a blank instruction").toEqual([]);
  // Still up, because closing on a refused press would read as having sent.
  await expect.element(page.getByRole("dialog")).toBeVisible();
});

test("whitespace is blank", async () => {
  const { sent } = opened();
  await userEvent.click(page.getByRole("button", { name: ACT_LABEL.redirect }));

  await userEvent.fill(page.getByRole("textbox", { name: "Instruction" }), "   \n  ");

  await expect.element(send()).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  expect(sent, "spaces counted as an instruction").toEqual([]);
});

test("an instruction satisfies it, and Enter then sends", async () => {
  const { sent } = opened();
  await userEvent.click(page.getByRole("button", { name: ACT_LABEL.redirect }));

  // **The half that keeps the two above honest.** A dialog that refused every
  // press would pass both of them, and the refusal is only correct if the same
  // key works the moment the field is filled.
  await userEvent.fill(
    page.getByRole("textbox", { name: "Instruction" }),
    "read root_cause.md before widening the catch",
  );
  await expect.element(send()).toBeEnabled();

  await userEvent.keyboard("{Enter}");
  expect(sent).toEqual([["job_2d90bb", "read root_cause.md before widening the catch"]]);
});
