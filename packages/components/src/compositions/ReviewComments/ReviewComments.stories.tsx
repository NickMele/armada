import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { ReviewComments } from "./ReviewComments";

/**
 * Somebody reviewed the pull request. This is where their comments reach a
 * drone, and where a person decides which of them should.
 *
 * Before this, the loop broke here: a reviewer left three comments, Armada did
 * nothing with them, and the person had to read them, decide which mattered and
 * then say all of it again somewhere Armada could hear — which is the retyping
 * the system exists to remove.
 *
 * **A person chooses, and that is the whole reason this is a surface.** Not
 * every comment is a change request; some are questions, some are agreement,
 * some are about something else. A drone handed all of them tries to satisfy
 * all of them.
 *
 * **Every word in a comment was written outside this machine.** It is drawn as
 * a text node and as nothing else — no markdown, no clickable link, no
 * attribute. The only value that goes back is the handle the forge gave it.
 *
 * **No glyph.** `packages/icons/icons.toml` has no mark for a review comment
 * and nothing there means one, so this draws none rather than borrowing a
 * silhouette that means something else.
 */
const meta: Meta<typeof ReviewComments> = {
  title: "Compositions/Review comments",
  component: ReviewComments,
};
export default meta;

type Story = StoryObj<typeof ReviewComments>;

/**
 * Three comments, none of them sent yet. Two are change requests and one is a
 * question about something else — which is the case this surface exists for,
 * and why nothing is preselected.
 */
export const NothingSentYet: Story = {
  args: {
    comments: [
      {
        id: "IC_kwDOfirst",
        by: "a-reviewer",
        at: "2026-09-08 10:00",
        said: "`held` reads backwards to me — it sounds like the worktree is being kept rather than that something is holding it. `holding` or `kept_by`?",
        takenUp: false,
      },
      {
        id: "IC_kwDOsecond",
        by: "a-reviewer",
        at: "2026-09-08 10:04",
        said: "There is no test for the case where the list is empty, and that is the one the panel draws differently.",
        takenUp: false,
      },
      {
        id: "IC_kwDOthird",
        by: "somebody-else",
        at: "2026-09-08 10:31",
        said: "Unrelated, but did the store migration ever land? I cannot see it on main.",
        takenUp: false,
      },
    ],
    onTakeUp: fn(),
  },
  /**
   * **Off until something is picked**, for the drone question's reason: Fleet
   * refuses an empty press, and a round trip to learn nothing was chosen is a
   * refusal that reads as a failure.
   *
   * What is sent is the handles, and only the ones picked. A regression to
   * "everything visible" would look right on a list where all three were
   * wanted, which is the list this surface exists because people do not have.
   */
  play: async ({ args, canvas, userEvent }) => {
    const send = canvas.getByRole("button", { name: "Send to a drone" });
    await expect(send).toBeDisabled();

    const picking = canvas.getAllByRole("checkbox", { name: "Act on this" });
    await userEvent.click(picking[0]!);
    await userEvent.click(picking[1]!);
    await expect(send).toBeEnabled();

    await userEvent.click(send);
    await expect(args.onTakeUp).toHaveBeenCalledWith(["IC_kwDOfirst", "IC_kwDOsecond"]);
  },
};

/**
 * One comment has already been handed to a drone and the other has not.
 *
 * **It is drawn rather than hidden.** The forge has no memory of what Armada
 * did, so the comment reads the same on the pull request forever; a list that
 * silently dropped it would leave a person looking at a review with holes in it
 * and wondering what happened to the rest. It cannot be picked, because Fleet
 * refuses a press naming a comment a drone already met — a drone that ran and
 * did not fully satisfy one must not meet it again as if it were new.
 */
export const OneAlreadySent: Story = {
  args: {
    comments: [
      {
        id: "IC_kwDOfirst",
        by: "a-reviewer",
        at: "2026-09-08 10:00",
        said: "`held` reads backwards to me. `holding` or `kept_by`?",
        takenUp: true,
      },
      {
        id: "IC_kwDOsecond",
        by: "a-reviewer",
        at: "2026-09-08 11:20",
        said: "The rename landed, thanks. The empty-list case still has no test though.",
        takenUp: false,
      },
    ],
    onTakeUp: fn(),
  },
  /** The sent one offers no control at all, so only one comment is choosable. */
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("checkbox", { name: "Act on this" })).toHaveLength(1);
    await expect(canvas.getByText("Already sent to a drone")).toBeVisible();
  },
};

/**
 * A pull request nobody has commented on.
 *
 * **Its own sentence, and never the one for a reading that failed.** Nobody has
 * said anything and nothing could be asked are different facts — the second is
 * the caller's to draw, because a person shown it as this one would conclude
 * their review had vanished.
 */
export const NobodyHasCommented: Story = {
  args: {
    comments: [],
    onTakeUp: () => {},
  },
};

/**
 * A comment that is four paragraphs, a fenced block and a line long enough to
 * widen anything it is drawn in.
 *
 * **Nothing is rendered and nothing is truncated.** The backticks stay
 * backticks and the heading stays a hash — a comment that could make its own
 * markup on this surface is a comment that could make a link. The paragraphs
 * survive because a reviewer's shape is part of what they said.
 */
export const ACommentWithEverythingInIt: Story = {
  args: {
    comments: [
      {
        id: "IC_kwDOlong",
        by: "a-reviewer",
        at: "2026-09-08 12:00",
        said: "# This is not a heading\n\nThe reader stops one line early. Reproduced with:\n\n```\narmada check bridge_test\n```\n\nSee https://example.invalid/an/extremely/long/path/that/keeps/going/and/going/and/going for the run.",
        takenUp: false,
      },
    ],
    onTakeUp: () => {},
  },
};

/**
 * One inline comment, with the code it is about, and one on the pull
 * request's own conversation, with a link.
 *
 * **The code comes before the words.** A person reading "this leaks a file
 * handle" wants the line it is about in view first. **The link opens through
 * `onOpenLink`, never through a `url` this surface holds** — the story's
 * `onOpenLink` stands in for main resolving the address again.
 */
export const OneInlineAndOneOnTheConversation: Story = {
  args: {
    comments: [
      {
        id: "PRRC_kwDOfirst",
        by: "a-reviewer",
        at: "2026-09-08 10:00",
        said: "This leaks a file handle on the early-return path above.",
        takenUp: false,
        hasLink: true,
        inline: {
          path: "src/log.rs",
          line: 42,
          hunk: "@@ -40,3 +40,3 @@ fn read() {\n     let file = File::open(path)?;\n-    Ok(file)\n+    Ok(file) // leaked on early return above\n",
        },
      },
      {
        id: "IC_kwDOsecond",
        by: "somebody-else",
        at: "2026-09-08 10:31",
        said: "Unrelated, but did the store migration ever land? I cannot see it on main.",
        takenUp: false,
        hasLink: true,
      },
    ],
    onTakeUp: fn(),
    onOpenLink: fn(),
    hostLabel: "GitHub",
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("src/log.rs:42")).toBeVisible();
    await expect(canvas.getByText(/leaked on early return above/)).toBeVisible();

    const links = canvas.getAllByRole("button", { name: /Open .*comment on GitHub/ });
    await expect(links).toHaveLength(2);
    await userEvent.click(links[0]!);
    await expect(args.onOpenLink).toHaveBeenCalledWith("PRRC_kwDOfirst");
  },
};

/**
 * Nothing live to send it over.
 *
 * **The reason is on screen**, because a disabled control with no sentence
 * beside it looks broken rather than held.
 */
export const NothingToSendItOver: Story = {
  args: {
    comments: [
      {
        id: "IC_kwDOfirst",
        by: "a-reviewer",
        at: "2026-09-08 10:00",
        said: "The empty-list case still has no test.",
        takenUp: false,
      },
    ],
    onTakeUp: () => {},
    disabled: true,
    disabledNote: "Fleet is not connected, so nothing here can be sent.",
  },
};
