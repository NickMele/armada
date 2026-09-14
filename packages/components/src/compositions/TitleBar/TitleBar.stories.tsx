import type { Meta, StoryObj } from "@storybook/react-vite";
import { Select } from "../../primitives/Select/Select";
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
    onDispatch: () => {},
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
