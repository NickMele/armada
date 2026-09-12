import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { DroneQuestion } from "./DroneQuestion";

/**
 * A drone that does not know, asking rather than guessing.
 *
 * The two moves it had before were escalate — which stops the job, freezes the
 * step and holds the worktree until somebody moves it — and guess, whose output
 * is drones spending on work nobody asked for. This is the third.
 *
 * **A drone's own question is a closed set; a command it was refused is not.**
 * Nothing is typed to answer a question, and free text there goes through
 * Redirect. A refusal carries its reason, because the reason exists at the
 * moment somebody presses reject and a second box spends it.
 *
 * **No glyph.** `icons.toml` has no mark for a drone asking.
 */
const meta: Meta<typeof DroneQuestion> = {
  title: "Compositions/Drone question",
  component: DroneQuestion,
};
export default meta;

type Story = StoryObj<typeof DroneQuestion>;

/** The field a refusal carries, and what a play finds it by. */
const NOTE_LABEL = "Note (optional)";

/** The control that asks what a command does, and what a play presses. */
const EXPLAIN = "Help me understand this command";

/**
 * The three answers Fleet offers on a command, with the reason riding on the
 * one that reads it. The other two tell the drone everything they commit to.
 */
const COMMAND_ANSWERS = [
  {
    label: "Allow for this job",
    consequence: "The drone runs it now, and this job can run it again without asking.",
  },
  {
    label: "Always allow in this repository",
    consequence:
      "The drone runs it now, and it is written into armada.yml under commands, as its own commit on this job's branch.",
  },
  {
    label: "Reject",
    consequence: "The drone is told no, and the command does not run.",
    noteLabel: NOTE_LABEL,
    noteSays: "Your words go to the drone with the refusal.",
  },
];

/**
 * The two-answer case, which is most of them. The question comes from building
 * the Focus milestone by hand on 30–31 Aug, where a guess would have been wrong
 * rather than merely presumptuous.
 */
export const TwoAnswers: Story = {
  args: {
    question:
      "The store schema needs a column before three of these jobs can run. Should that be its own job?",
    options: [
      {
        label: "Its own job",
        consequence:
          "Dispatch a migration job first and make the other three depend on it. Nothing else starts until it lands.",
      },
      {
        label: "Fold it in",
        consequence:
          "The first job that needs the column adds it. The other two may race it and one of them will have to wait anyway.",
      },
    ],
    waiting: "12m",
    onAnswer: fn(),
  },
  /**
   * **Off until something is picked.** Fleet would refuse an empty answer, and
   * a round trip to learn nothing was chosen is a refusal that reads as a
   * failure — so the control never offers the press in the first place.
   *
   * The answer sent is the label, which is what the drone asked with. A
   * regression to an index would look right here and be wrong the moment Fleet
   * reordered the options.
   */
  play: async ({ args, canvas, userEvent }) => {
    const send = canvas.getByRole("button", { name: "Send this answer" });
    await expect(send).toBeDisabled();

    await userEvent.click(canvas.getByRole("radio", { name: "Its own job" }));
    await expect(send).toBeEnabled();

    await userEvent.click(send);
    await expect(args.onAnswer).toHaveBeenCalledWith("Its own job");
  },
};

/**
 * Four, which is the most a question may offer. Fleet refuses a fifth: the
 * whole value of asking rather than escalating is that a person answers in one
 * glance, and a list long enough to scroll is a list read badly at 11pm.
 */
export const FourAnswers: Story = {
  args: {
    question: "How should this milestone's work be split?",
    options: [
      {
        label: "By crate",
        consequence: "One job per crate that changes — six jobs, each with its own scope.",
      },
      {
        label: "By milestone step",
        consequence:
          "One job per step of the milestone as written — four jobs, two of which cross three crates.",
      },
      {
        label: "By side of the seam",
        consequence: "Two jobs: everything in Rust, and everything in Bridge.",
      },
      {
        label: "One job",
        consequence: "Do not split it. One drone works the whole milestone in one worktree.",
      },
    ],
    waiting: "2h",
    onAnswer: () => {},
  },
};

/**
 * Just asked. **No elapsed at all rather than `0m`** — a zero is a measurement
 * and this is the absence of one, and the two read differently to somebody
 * scanning for how long something has been sitting.
 */
export const JustAsked: Story = {
  args: {
    question: "Two issues in this milestone contradict each other on the gate. Which one holds?",
    options: [
      {
        label: "The Judge decides",
        consequence: "Gate the step on the Judge, and drop the human gate from it.",
      },
      {
        label: "A person decides",
        consequence: "Keep the human gate, and the step stops for somebody every time.",
      },
    ],
    onAnswer: () => {},
  },
};

/**
 * **A command the drone reached for and was not given**, on a job set to Ask
 * me. The same box, because it is the same moment: a drone stopped inside a
 * call, a closed set of answers, and a person who has to pick one.
 *
 * The command is what is asked, in mono inside the sentence, because it is
 * what the drone sent. The three answers are the whole set Fleet offers, so the
 * line under the control says what an answer given late still does.
 */
export const ACommandItWasNotGiven: Story = {
  args: {
    question: (
      <>
        The drone wants to run <span className="mono">pnpm add -D reselect@5.1.1</span>
      </>
    ),
    options: COMMAND_ANSWERS,
    waiting: "2m",
    redirectNote:
      "If the drone stops waiting before you answer, it is told to hold, and your answer reaches it as its next turn.",
    onAnswer: () => {},
  },
};

/**
 * A refusal carrying its reason.
 *
 * **The field is under the one answer that reads it**, and an allow never grows
 * one: allowing tells the drone everything it acts on, so there is nothing left
 * to say. The words go with the answer rather than after it.
 */
export const ARefusalWithAReason: Story = {
  args: { ...ACommandItWasNotGiven.args, onAnswer: fn() } as Story["args"],
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByLabelText(NOTE_LABEL)).toBeNull();

    // An allow needs no reason, and is offered no field.
    await userEvent.click(canvas.getByRole("radio", { name: "Allow for this job" }));
    await expect(canvas.queryByLabelText(NOTE_LABEL)).toBeNull();

    await userEvent.click(canvas.getByRole("radio", { name: "Reject" }));
    await userEvent.type(canvas.getByLabelText(NOTE_LABEL), "reselect is not a dependency we want");
    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    await expect(args.onAnswer).toHaveBeenCalledWith(
      "Reject",
      "reselect is not a dependency we want",
    );
  },
};

/**
 * A refusal sent with nothing typed. **Nothing rather than an empty string** —
 * a bare refusal is what every fleet before protocol 11.5 sent, and a blank
 * note would be a person's words that nobody wrote.
 */
export const ARefusalWithNothingSaid: Story = {
  args: { ...ACommandItWasNotGiven.args, onAnswer: fn() } as Story["args"],
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("radio", { name: "Reject" }));
    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("Reject", undefined);
  },
};

/** The command with a reading on offer and nobody having asked for one yet. */
export const ACommandToRead: Story = {
  args: {
    ...ACommandItWasNotGiven.args,
    explain: { state: "ready" },
    onExplain: fn(),
  } as Story["args"],
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: EXPLAIN }));
    await expect(args.onExplain).toHaveBeenCalled();
  },
};

/**
 * The reading, with the model that gave it named.
 *
 * **It decided nothing**, which is what the second half of this play reads: the
 * three answers are live after it lands, exactly as they were before anybody
 * pressed.
 */
export const ACommandRead: Story = {
  args: {
    ...ACommandItWasNotGiven.args,
    explain: {
      state: "read",
      explanation:
        "It adds reselect 5.1.1 to this repository as a development dependency and writes the lockfile. It reaches the network, and it changes two files a review would see.",
      model: "haiku",
    },
  } as Story["args"],
  play: async ({ canvas }) => {
    const reading = within(canvas.getByRole("status", { name: "What this command does" }));
    await expect(reading.getByText(/adds reselect 5\.1\.1/)).toBeVisible();
    await expect(reading.getByText("haiku")).toBeVisible();

    await expect(canvas.getByRole("radio", { name: "Reject" })).toBeEnabled();
    await expect(canvas.getByRole("radio", { name: "Allow for this job" })).toBeEnabled();
  },
};

/**
 * The reading is out. **The answers stay live while it is**, because asking what
 * a command does is not a step on the way to answering — a person who has
 * already decided should not wait for a model they stopped needing.
 */
export const ACommandBeingRead: Story = {
  args: {
    ...ACommandItWasNotGiven.args,
    explain: { state: "asking" },
    onExplain: fn(),
  } as Story["args"],
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Reading this command.");
    // The one control that is off is the one whose own request is out.
    await expect(canvas.getByRole("button", { name: EXPLAIN })).toBeDisabled();
    await expect(canvas.getByRole("radio", { name: "Reject" })).toBeEnabled();
  },
};

/**
 * The reading did not arrive. **Said plainly, and the answers are untouched** —
 * a person who never asked for a reading is answered exactly as before, so one
 * that failed must not take the decision down with it.
 */
export const ACommandThatWouldNotRead: Story = {
  args: {
    ...ACommandItWasNotGiven.args,
    explain: { state: "failed", why: "Fleet did not explain this command." },
    onAnswer: fn(),
  } as Story["args"],
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("Fleet did not explain this command.")).toBeVisible();

    await userEvent.click(canvas.getByRole("radio", { name: "Allow for this job" }));
    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    await expect(args.onAnswer).toHaveBeenCalledWith("Allow for this job");
  },
};

/**
 * An answer already in flight. **The reason is on the surface**, because a
 * disabled control with no sentence beside it reads as an app that is broken
 * rather than one that is busy.
 */
export const Sending: Story = {
  args: {
    ...TwoAnswers.args,
    disabled: true,
    disabledNote: "That answer is already on its way to the drone.",
  } as Story["args"],
};

/**
 * Nothing live to send over. The same controls, a different sentence — and the
 * question is still legible, because reading what was asked does not need a
 * connection.
 */
export const NotConnected: Story = {
  args: {
    ...TwoAnswers.args,
    disabled: true,
    disabledNote: "Fleet is not connected, so nothing can be sent. The drone is still waiting.",
  } as Story["args"],
};
