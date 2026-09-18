// The message box's own gate: `steeringOf`, and only it — #1154.
//
// **A browser test, not a plain one.** `DroneMessageControl` owns its own
// `useState`, the same reason `Redirect.test.tsx` mounts `RedirectControl`
// rather than calling a pure function.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";

import { DroneMessageControl } from "./DroneMessage";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const JOB_ID = "01M22TYSAE0023MADDP5ZQEYGW";

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: JOB_ID,
    handle: "9-message-a-working-drone",
    title: "Message a working drone",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-15T09:00:00Z",
    assigned_drone: "01M1HHJ6XB001BZJZ4BE2XKY34",
    ...over,
  };
}

function whole(over: Partial<JobWhole> = {}): JobWhole {
  return {
    job: job(),
    created_at: "2026-09-15T09:00:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    ...over,
  };
}

test("a working drone takes a message, and the field clears once it is sent", async () => {
  const sent: [string, string][] = [];
  mount(
    <DroneMessageControl
      job={job()}
      whole={whole()}
      onRedirect={(jobId, instruction) => sent.push([jobId, instruction])}
    />,
  );
  const field = page.getByRole("textbox", { name: "Message the drone" });
  await userEvent.fill(field, "Check the second failing test too");
  await userEvent.click(page.getByRole("button", { name: "Send" }));
  expect(sent).toEqual([[JOB_ID, "Check the second failing test too"]]);
  await expect.element(field).toHaveValue("");
});

// The binding is `send_message` in the registry and the box's own keydown; what
// this adds is that it reaches the redirect and clears the field, which is the
// wiring rather than the key.
test("⌘Enter sends the redirect, and plain Enter writes a second line", async () => {
  const sent: [string, string][] = [];
  mount(
    <DroneMessageControl
      job={job()}
      whole={whole()}
      onRedirect={(jobId, instruction) => sent.push([jobId, instruction])}
    />,
  );
  const field = page.getByRole("textbox", { name: "Message the drone" });
  await userEvent.fill(field, "Check the second failing test too");
  await userEvent.keyboard("{Enter}");
  expect(sent).toEqual([]);

  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  expect(sent).toEqual([[JOB_ID, "Check the second failing test too\n"]]);
  await expect.element(field).toHaveValue("");
});

test("no drone on the step disables the box and says why", async () => {
  mount(
    <DroneMessageControl
      job={job({ assigned_drone: undefined })}
      whole={whole()}
      onRedirect={() => {}}
    />,
  );
  await expect.element(page.getByRole("textbox", { name: "Message the drone" })).toBeDisabled();
  await expect.element(page.getByRole("button", { name: "Send" })).toBeDisabled();
  await expect
    .element(page.getByText("No drone is on this step, so there's nothing to send this to."))
    .toBeVisible();
});

test("a redirect already out shows in the box, and the field stays open to replace it", async () => {
  mount(
    <DroneMessageControl
      job={job()}
      whole={whole({ redirecting: { sent_at: "2026-09-15T14:41:00Z" } })}
      onRedirect={() => {}}
    />,
  );
  await expect.element(page.getByText("Sent, waiting for the drone.", { exact: false })).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Message the drone" })).toBeEnabled();
});

// Escalated or otherwise stopped: the drone that reads a redirect there is
// `recovery.ts`'s "holding", not this box's "working" — the button and its
// dialog stay the only way to reach it. #1154's own scope.
test("an escalated job disables the box, the same as no drone at all", async () => {
  mount(
    <DroneMessageControl job={job({ status: "escalated" })} whole={whole()} onRedirect={() => {}} />,
  );
  await expect.element(page.getByRole("textbox", { name: "Message the drone" })).toBeDisabled();
});
