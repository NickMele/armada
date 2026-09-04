import type { Meta, StoryObj } from "@storybook/react-vite";

import { ManifestNotice } from "./ManifestNotice";

/**
 * What Fleet's last read of `armada.yml` came to, drawn where a person will
 * meet it: above the surface, persistent, and put away by hand.
 *
 * **The states below are the readings Fleet can actually hold**, not states
 * invented for the drawing. A read either took, took in part, or was refused;
 * `crates/ipc/src/reading.rs` is the shape and `config::Adopted` and
 * `config::LoadError` are what fills it.
 *
 * A reading with nothing in it — a saved comment, an editor writing the same
 * bytes back — has no story, because nothing draws one. That judgement is
 * `worthSaying`, and the surface asks it before it renders this at all.
 */
const meta: Meta<typeof ManifestNotice> = {
  title: "Compositions/Manifest notice",
  component: ManifestNotice,
};
export default meta;

type Story = StoryObj<typeof ManifestNotice>;

const AT = "2026-09-04T09:12:04.000Z";

/**
 * The edit took. Neutral, because nothing is wrong — a value moved and Fleet is
 * running with it.
 *
 * Both ends of the move are drawn. "The poke limit changed" is a sentence a
 * person still has to open the file to act on; "is 5, was 3" is one they do not.
 */
export const Reloaded: Story = {
  args: {
    reading: {
      path: "armada.yml",
      at: AT,
      moved: [{ key: "drone.poke_limit", before: 3, after: 5 }],
    },
    onDismiss: () => undefined,
  },
};

/**
 * A key that was not in the file before. **`unset` is spelled rather than left
 * blank**: an absent key is the repository deferring to what Fleet runs with,
 * which is a different fact from a number, and a gap would read as a value that
 * failed to load.
 */
export const ReloadedFromUnset: Story = {
  args: {
    reading: {
      path: "armada.yml",
      at: AT,
      moved: [{ key: "drone.quiet_after_seconds", after: 900 }],
    },
    onDismiss: () => undefined,
  },
};

/**
 * The refusal, which is the state this whole surface exists for.
 *
 * **Three things, in the order a person needs them**: the sentence Fleet
 * refused it with, the keys it refused, and — the one that stops somebody
 * editing the file twice more before noticing — that the previous values are
 * still running.
 *
 * Every fault crosses, not the first. Correcting a file from a message naming
 * one fault means saving, waiting, and meeting the next.
 */
export const Refused: Story = {
  args: {
    reading: {
      path: "armada.yml",
      at: AT,
      refused: {
        summary: "armada.yml was refused; `drone.poke_limit` is not a number",
        faults: [
          { key: "drone.poke_limit", fault: "is not a number" },
          { key: "checks.build.run", fault: "is required and absent" },
        ],
      },
    },
    onDismiss: () => undefined,
  },
};

/**
 * A file that is not YAML at all. **No keys, because there is no document to
 * have keys** — so the parser's own sentence, which carries the line and the
 * column, is the whole answer.
 *
 * This is the reading the issue was written about: "line 14 is not a number" is
 * what gets somebody to the edit, and it can only come from here.
 */
export const RefusedBeforeItParsed: Story = {
  args: {
    reading: {
      path: "armada.yml",
      at: AT,
      refused: {
        summary: "armada.yml is not YAML: mapping values are not allowed in this context at line 14 column 18",
      },
    },
    onDismiss: () => undefined,
  },
};

/**
 * A section that changed and was not adopted. **The quietest failure of the
 * three and the one most worth saying**: the file parsed, so nothing refused
 * it, and yet the behaviour and the file disagree until Fleet is restarted.
 *
 * Somebody who edits `checks:` under a running Fleet and is told nothing has no
 * way to tell that from it having worked.
 */
export const ChangedUntilRestart: Story = {
  args: {
    reading: {
      path: "armada.yml",
      at: AT,
      at_restart: ["checks", "setup"],
    },
    onDismiss: () => undefined,
  },
};
