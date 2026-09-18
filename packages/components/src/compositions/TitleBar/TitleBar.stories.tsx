import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { FLEET_DOT_TONE, type FleetState } from "../FleetPanel/FleetPanel";
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
 * The left column is folded, so Fleet's dot is on the bar's trailing edge —
 * the one width it is drawn at (#1437). It says liveness and no more: pid,
 * port, protocol and uptime come back with the panel.
 */
export const FleetFolded: Story = {
  args: {
    ...Full.args,
    helm: { questions: 3, binding: "⌘J", onOpen: () => {} },
    fleet: { state: "running", label: "Running" },
  },
};

/** Fleet is not running — `--status-escalated`, the panel's own hue for it, and the word in the name. */
export const FleetFoldedNotRunning: Story = {
  args: { ...FleetFolded.args, fleet: { state: "not-running", label: "Not running" } },
};

/** Alive and not answering — `--status-awaiting-review`, a state a dot alone could not tell from running. */
export const FleetFoldedUnreachable: Story = {
  args: { ...FleetFolded.args, fleet: { state: "unreachable", label: "Unreachable" } },
};

/** None of the three — reading, connecting, a refused runtime file — neutral, and the label names which. */
export const FleetFoldedUnknown: Story = {
  args: { ...FleetFolded.args, fleet: { state: "unknown", label: "Connecting" } },
};

/**
 * What a rendering cannot show: that each state's dot carries the Fleet
 * panel's own tone rather than one chosen here, and that the state is
 * readable without sight. The tone is asserted against `FLEET_DOT_TONE`
 * itself — the map the panel draws from — so a hue that moved in one place
 * and not the other fails here rather than shipping two dots that disagree.
 *
 * The name and the tooltip are read as two properties because they are two
 * doors: a pointer gets `title`, a screen reader gets the accessible name,
 * and a dot that had only one of them would be silent for half its readers.
 */
export const FleetStates: Story = {
  args: FleetFolded.args,
  render: (args) => (
    <>
      {FLEET_SAID.map(({ state, label }) => (
        <TitleBar key={state} {...args} fleet={{ state, label }} />
      ))}
    </>
  ),
  play: async ({ canvas }) => {
    for (const { state, label } of FLEET_SAID) {
      const said = `Fleet — ${label}`;
      const dot = canvas.getByRole("img", { name: said });
      await expect(dot).toHaveAttribute("title", said);
      await expect(dot.firstElementChild).toHaveAttribute("data-tone", FLEET_DOT_TONE[state]);
    }
  },
};

/** Every Fleet state, with the word the panel puts beside its own dot for each. */
const FLEET_SAID: { state: FleetState; label: string }[] = [
  { state: "running", label: "Running" },
  { state: "not-running", label: "Not running" },
  { state: "unreachable", label: "Unreachable" },
  { state: "unknown", label: "Connecting" },
];

/** Unfolded: the Fleet panel carries the dot, and this row draws none. Two would be one fact twice. */
export const FleetUnfolded: Story = {
  args: { ...Full.args, helm: { questions: 3, binding: "⌘J", onOpen: () => {} } },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("img", { name: /^Fleet — / })).not.toBeInTheDocument();
  },
};

/**
 * Holding Cmd mounts Helm's own `⌘J` badge, growing the button to fit it —
 * shrinking it back and taking the badge away again on release — the row is
 * drawn from `Full` and never itself. `useShortcutReveal()` needs a provider
 * above it for the state to reach anywhere at all, which `TheShell` is in the
 * running app and this story stands in for here.
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
    const findBadge = () =>
      canvas.queryByText((_, el) => el?.tagName === "KBD" && el.textContent === "⌘J");
    await expect(findBadge()).not.toBeInTheDocument();

    await userEvent.keyboard("{Meta>}");
    await expect(findBadge()).toBeVisible();

    await userEvent.keyboard("{/Meta}");
    await expect(findBadge()).not.toBeInTheDocument();
  },
};
