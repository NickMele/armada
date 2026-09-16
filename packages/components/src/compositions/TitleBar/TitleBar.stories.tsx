import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { Select } from "../../primitives/Select/Select";
import { ShortcutRevealProvider } from "../../shortcut-reveal";
import { TitleBar } from "./TitleBar";

/**
 * The title row #1087 put in place of macOS's grey bar. Every story below
 * sits in the same framed strip `TheShell`'s own stories use, at the
 * drawing's own screen width.
 */
const meta: Meta<typeof TitleBar> = {
  title: "Compositions/TitleBar",
  component: TitleBar,
  decorators: [
    // A column, matching `.armada-shell`'s own axis — that is what stretches
    // the bar to the frame's width there, and a row (`.armada-screen__window`)
    // would only shrink-wrap it to its content, which is a story artefact
    // nothing in Bridge ever draws.
    (Story) => (
      <div className="armada-screen" style={{ display: "flex", flexDirection: "column" }}>
        <div className="armada-screen__window" style={{ height: "auto", flexDirection: "column" }}>
          <Story />
        </div>
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof TitleBar>;

const picker = (
  <Select aria-label="Project">
    <option>All repositories</option>
  </Select>
);

/** Every slot filled: the picker, the search field, Dispatch and the logo — Helm's dock is open, so its button is absent. */
export const Full: Story = {
  args: {
    repositoryPicker: picker,
    onSearch: () => {},
    onDispatch: fn(),
  },
  /**
   * #1156: Dispatch is one button with no caret, and clicking it dispatches.
   * `SplitButton` drew a second, separately-named control for the menu even
   * with nothing behind it — `queryByRole` for that name is the regression
   * this guards.
   */
  play: async ({ args, canvas, userEvent }) => {
    const dispatch = canvas.getByRole("button", { name: "Dispatch" });
    await expect(canvas.queryByRole("button", { name: "Dispatch a job" })).not.toBeInTheDocument();

    await userEvent.click(dispatch);
    await expect(args.onDispatch).toHaveBeenCalledTimes(1);
  },
};

/** The dock is closed: Helm's own reopen control appears before the logo, carrying what is waiting. */
export const HelmClosed: Story = {
  args: {
    ...Full.args,
    helm: { questions: 3, binding: "⌘J", onOpen: () => {} },
  },
};

/** Nothing connected yet: no picker, no Dispatch — the bar still draws, with the logo and nowhere else to go but search. */
export const BeforeAnythingIsRead: Story = {
  args: {
    onSearch: () => {},
  },
};

/**
 * Holding Cmd grows Helm's own `⌘J` badge without moving the button beside
 * it, and releasing it takes the badge away again — the row is drawn from
 * `Full` and never itself. `useShortcutReveal()` needs a provider above it
 * for the state to reach anywhere at all, which `TheShell` is in the running
 * app and this story stands in for here.
 */
export const RevealedShortcuts: Story = {
  args: {
    ...Full.args,
    helm: { questions: 3, binding: "⌘J", onOpen: () => {} },
  },
  render: (args) => (
    <ShortcutRevealProvider>
      <TitleBar {...args} />
    </ShortcutRevealProvider>
  ),
  play: async ({ canvas, userEvent }) => {
    const badge = canvas.getByText((_, el) => el?.tagName === "KBD" && el.textContent === "⌘J");
    await expect(badge).not.toBeVisible();

    await userEvent.keyboard("{Meta>}");
    await expect(badge).toBeVisible();

    await userEvent.keyboard("{/Meta}");
    await expect(badge).not.toBeVisible();
  },
};
