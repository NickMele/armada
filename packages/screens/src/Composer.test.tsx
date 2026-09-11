// The hand-entry form's own attachment and mention wiring.
//
// **Not `DispatchRequest`'s claim again.** That component's stories prove the
// mechanics against its own field — a picked file stages as a chip, typing
// `@` opens a popup, picking a result inserts it. This proves the same two
// paths wired into a different field on a different component: `Composer`'s
// Brief, which shares `useMention` and `stage()` by construction and not by
// having been copy-pasted correctly. And it proves the one thing no story
// can: that what gets staged here actually rides the `Draft` `propose()`
// sends, and is cleared once it does.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { Composer } from "./Composer";
import type { Draft, ManifestSummary, WorkflowSummary } from "@armada/protocol";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const A_WORKFLOW: WorkflowSummary = {
  id: "wf_bug",
  name: "bug",
  version: 1,
  steps: [],
  manifest_id: "mf_1",
};

const A_MANIFEST: ManifestSummary = {
  id: "mf_1",
  repository: "armada",
  path: "/repo",
  records_root: "/repo/.armada",
  version: 1,
  checks: [],
};

/** Mount it, and hand back every draft `propose()` sent. */
function opened(): { drafts: Draft[] } {
  const drafts: Draft[] = [];
  mount(
    <Composer
      workflows={[A_WORKFLOW]}
      manifest={A_MANIFEST}
      models={{ models: ["a-model"], default: "a-model" }}
      disabled={false}
      onStage={(_bytes, filename, mimeType) =>
        Promise.resolve({ path: `/tmp/staged/${filename}-${mimeType}` })
      }
      onSearchFiles={(query) =>
        Promise.resolve(
          ["README.md", "packages/components/README.md"].filter((path) =>
            path.toLowerCase().includes(query.toLowerCase()),
          ),
        )
      }
      onPropose={(draft) => drafts.push(draft)}
    />,
  );
  return { drafts };
}

function brief() {
  return page.getByRole("textbox", { name: "Brief" });
}

/**
 * The "Attach" button reaches a hidden `<input type="file">` — the button
 * only opens a native picker this suite cannot drive, so the file is handed
 * straight to the input the same way a real pick reaches it, through the
 * platform's own chrome. What stages it from there is `Composer`'s own
 * `stage()`, the same function a pasted screenshot calls.
 */
test("the attach button stages a picked file, and it rides the draft", async () => {
  const { drafts } = opened();
  const picked = new File(["diagnostic output"], "log.txt", { type: "text/plain" });

  await userEvent.upload(page.getByTestId("composer-attach-input"), picked);
  await expect.element(page.getByRole("button", { name: "Remove log.txt" })).toBeVisible();

  await userEvent.fill(page.getByRole("textbox", { name: "Title" }), "Fix the flicker");
  await userEvent.fill(brief(), "The board flickers on every event.");
  await userEvent.click(page.getByRole("button", { name: "Propose" }));

  expect(drafts).toHaveLength(1);
  expect(drafts[0]!.attachments).toEqual([
    { path: "/tmp/staged/log.txt-text/plain", filename: "log.txt", mimeType: "text/plain" },
  ]);
  // Cleared once sent, the same way the title and the brief are.
  await expect.element(page.getByRole("button", { name: "Remove log.txt" })).not.toBeInTheDocument();
});

/** A staged chip's own remove button drops it, and only it. */
test("removing a chip drops that attachment and no other", async () => {
  opened();
  const before = new File(["a"], "before.png", { type: "image/png" });
  const after = new File(["b"], "after.png", { type: "image/png" });

  await userEvent.upload(page.getByTestId("composer-attach-input"), [before, after]);
  await expect.element(page.getByRole("button", { name: "Remove after.png" })).toBeVisible();

  await userEvent.click(page.getByRole("button", { name: "Remove before.png" }));

  await expect.element(page.getByRole("button", { name: "Remove before.png" })).not.toBeInTheDocument();
  await expect.element(page.getByRole("button", { name: "Remove after.png" })).toBeVisible();
});

/**
 * Typing `@` opens the mention popup over the Brief field, and picking a
 * result inserts it at the `@` — `Composer` wires the same `useMention` the
 * Request field does, against its own text and its own setter.
 */
test("an at-mention in the brief opens the popup and inserts the pick", async () => {
  opened();
  await userEvent.fill(brief(), "See @READ");

  await userEvent.click(page.getByRole("option", { name: "README.md", exact: true }));

  await expect.element(brief()).toHaveValue("See @README.md ");
});
