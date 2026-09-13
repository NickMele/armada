import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { ManifestFile } from "./ManifestFile";

/**
 * Journey 9's *Editing*, the file half: `armada.yml` as text, one Save, and
 * what Fleet's next reading of the file came to.
 *
 * **Every state below is one the wire can produce.** The text is what
 * `GET /manifest/file` answers, a save is `POST /manifest/save_file`, the
 * reading is `manifest.reread`, and a file that moved is the 409 Fleet raises
 * under `fleet.manifest_moved_under_the_edit` with `on_disk` riding it.
 *
 * The file fills the panel it is mounted in, so the story draws a
 * viewport-high column — `Compositions/Run page`'s mount.
 */
const meta: Meta<typeof ManifestFile> = {
  title: "Compositions/Manifest file",
  component: ManifestFile,
  decorators: [
    (Story) => (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          height: "calc(100dvh - 2 * var(--space-6))",
          background: "var(--bg-base)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof ManifestFile>;

const PATH = "/Users/user/armada/armada.yml";

/** A cut of this repository's own Manifest — enough to read as the file it is. */
const READ = `version: 1
id: armada
base: main

drone:
  poke_limit: 3

checks:
  build:
    run: cargo build --workspace --locked
  test:
    run: cargo nextest run --workspace --exclude acceptance
  typecheck:
    run: pnpm typecheck
`;

/** The correction a person made: a higher poke limit and a mistyped Check. */
const EDITED = READ.replace("poke_limit: 3", "poke_limit: 5").replace(
  "    run: pnpm typecheck",
  "    runs: pnpm typecheck",
);

/** What a `git pull` brought in while the edit was open. */
const PULLED = READ.replace(
  "  typecheck:\n    run: pnpm typecheck\n",
  "  typecheck:\n    run: pnpm typecheck\n  bridge_build:\n    run: pnpm -C apps/desktop build\n",
);

/** Opened, nothing typed. Save is not offered for bytes already on disk. */
export const Opened: Story = {
  args: { path: PATH, text: READ, changed: false, onText: fn(), onSave: fn() },
};

/** Edited and not saved yet. */
export const Edited: Story = {
  args: { path: PATH, text: EDITED, changed: true, onText: fn(), onSave: fn() },
};

/**
 * Saved, and Fleet has not read it yet. **The reading on hand is held back** —
 * it is of the file before the save, and a refusal already corrected must not
 * read as the answer to the correction.
 */
export const Settling: Story = {
  args: {
    path: PATH,
    text: EDITED,
    changed: false,
    saved: { at: "14:20:03", settled: false },
    reading: {
      path: PATH,
      at: "2026-09-12T14:11:40.000Z",
      refused: { summary: "armada.yml could not be adopted: 1 fault", faults: [{ key: "checks.test.run", fault: "is empty" }] },
    },
    onText: fn(),
    onSave: fn(),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText(/Fleet reads it once the file settles/)).toBeVisible();
    // The refusal belongs to the file before this save, so nothing of it shows.
    await expect(canvas.queryByText("checks.test.run")).toBeNull();
  },
};

/** Saved, read, and a live key moved. */
export const ASaveThatTook: Story = {
  name: "A save that took",
  args: {
    path: PATH,
    text: READ.replace("poke_limit: 3", "poke_limit: 5"),
    changed: false,
    saved: { at: "14:20:03", settled: true },
    reading: {
      path: PATH,
      at: "2026-09-12T14:20:05.000Z",
      moved: [{ key: "drone.poke_limit", before: "3", after: "5" }],
    },
    onText: fn(),
    onSave: fn(),
  },
};

/** Saved and read, and nothing Fleet reads while running changed. */
export const ASaveWithNothingToSay: Story = {
  name: "A save with nothing to say",
  args: {
    path: PATH,
    text: READ,
    changed: false,
    saved: { at: "14:20:03", settled: true },
    reading: { path: PATH, at: "2026-09-12T14:20:05.000Z" },
    onText: fn(),
    onSave: fn(),
  },
};

/**
 * Saved, and Fleet refused what it read. **Why first, then that the values in
 * force are unchanged** — and every key it was refused for, not the first.
 */
export const ARefusedSave: Story = {
  name: "A refused save",
  args: {
    path: PATH,
    text: EDITED,
    changed: false,
    saved: { at: "14:20:03", settled: true },
    reading: {
      path: PATH,
      at: "2026-09-12T14:20:05.000Z",
      refused: {
        summary: "armada.yml could not be adopted: 2 faults",
        faults: [
          { key: "checks.typecheck", fault: "unknown field `runs`, expected `run`" },
          { key: "checks.typecheck.run", fault: "is required" },
        ],
      },
    },
    onText: fn(),
    onSave: fn(),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Manifest refused")).toBeVisible();
    await expect(canvas.getByText("checks.typecheck")).toBeVisible();
    await expect(canvas.getByText("checks.typecheck.run")).toBeVisible();
    await expect(canvas.getByText(/The values in force are unchanged/)).toBeVisible();
    // Bytes already on disk are not offered for saving again.
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
  },
};

/** A document that never became one — the only place a line number appears. */
export const NotADocument: Story = {
  name: "Not a document",
  args: {
    path: PATH,
    text: READ.replace("base: main", "base: main:\n  - ["),
    changed: false,
    saved: { at: "14:20:03", settled: true },
    reading: {
      path: PATH,
      at: "2026-09-12T14:20:05.000Z",
      refused: { summary: "armada.yml is not YAML: did not find expected node content at line 5 column 6" },
    },
    onText: fn(),
    onSave: fn(),
  },
};

/**
 * A pull landed while the edit was open, and Fleet refused the save rather
 * than taking it with it. **Both texts, side by side, and nothing overwritten**
 * unless the control that says so is pressed.
 */
export const ASaveOverAFileThatMoved: Story = {
  name: "A save over a file that moved",
  args: {
    path: PATH,
    text: EDITED,
    changed: true,
    moved: { onDisk: PULLED, onSaveOver: fn(), onTakeOnDisk: fn() },
    onText: fn(),
    onSave: fn(),
  },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("The file changed after you opened it")).toBeVisible();
    await expect(canvas.getByRole("textbox", { name: "On disk now" })).toHaveValue(PULLED);
    await expect(canvas.getByRole("textbox", { name: "Your edit" })).toHaveValue(EDITED);
    // The plain Save is gone: the only save left is the one that says it overwrites.
    await expect(canvas.queryByRole("button", { name: "Save" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Save my edit over it" }));
    await expect(args.moved?.onSaveOver).toHaveBeenCalledOnce();
    await expect(args.moved?.onTakeOnDisk).not.toHaveBeenCalled();
  },
};

/** The file was removed under the edit. Fleet will not put it back. */
export const TheFileIsGone: Story = {
  name: "The file is gone",
  args: {
    path: PATH,
    text: EDITED,
    changed: true,
    moved: { onDisk: null, onReadAgain: fn() },
    onText: fn(),
    onSave: fn(),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("textbox", { name: "Your edit" })).toHaveValue(EDITED);
    await expect(canvas.queryByRole("button", { name: "Save my edit over it" })).toBeNull();
  },
};

/** Fleet would not write the bytes. The edit stays where it was. */
export const NotWritten: Story = {
  name: "Not written",
  args: {
    path: PATH,
    text: EDITED,
    changed: true,
    failure: `the corrected Manifest could not be written to ${PATH}: Permission denied (os error 13). Nothing was changed, so what is on disk is what was there before`,
    onText: fn(),
    onSave: fn(),
  },
};
