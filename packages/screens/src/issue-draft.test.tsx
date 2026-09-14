// The issue draft's keyboard: Enter in the body writes a new line, and nothing is filed until a
// person confirms. #906.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { IssueDraftDialog } from "./issue-draft";
import { mount, unmount } from "./mounted";

afterEach(unmount);

/** Mount the draft, and hand back every title and body it filed. */
function opened(): { filed: [string, string][] } {
  const filed: [string, string][] = [];
  mount(
    <IssueDraftDialog
      draft={{
        finding: "The retry count is a guess",
        title: "The retry count is a guess",
        body: "Nothing measured it.",
      }}
      onFile={(title, body) => filed.push([title, body])}
      onCancel={() => undefined}
    />,
  );
  return { filed };
}

test("Enter in the body writes a new line rather than filing", async () => {
  const { filed } = opened();
  await userEvent.fill(page.getByRole("textbox", { name: "Body" }), "Nothing measured it.");
  await userEvent.keyboard("{Enter}More.");
  expect(filed).toEqual([]);

  await userEvent.click(page.getByRole("button", { name: /File the issue/ }));
  expect(filed).toEqual([["The retry count is a guess", "Nothing measured it.\nMore."]]);
});

test("a blank title cannot be filed", async () => {
  const { filed } = opened();
  await userEvent.fill(page.getByRole("textbox", { name: "Title" }), "   ");
  await expect.element(page.getByRole("button", { name: /File the issue/ })).toBeDisabled();
  expect(filed).toEqual([]);
});
