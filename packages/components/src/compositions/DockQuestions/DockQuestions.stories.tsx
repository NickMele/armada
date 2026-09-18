import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn } from "storybook/test";

import { DockQuestions, type DockQuestion } from "./DockQuestions";

/**
 * The questions zone of Helm's dock, drawn at the dock's own width on its sunken ground. One story
 * per kind of question, and one with questions from two repositories.
 */
const meta: Meta<typeof DockQuestions> = {
  title: "Compositions/Dock questions",
  component: DockQuestions,
  render: (args) => (
    <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
      <DockQuestions {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof DockQuestions>;

const droneQuestion: DockQuestion = {
  id: "01M21BKVPW002DC0ATD1X9T0VF:drone",
  repository: "armada",
  job: "12",
  title: "The drone count is wrong after a restart",
  label: "The drone asked a question",
  asked: "Should the count include drones that exited during the restart, or only those still running?",
  waiting: "4m",
  answers: [
    { id: "Only running", label: "Only running", consequence: "Counts drones with a live process and nothing else." },
    { id: "Include exited", label: "Include exited", consequence: "Keeps exited drones in the count until the next status move." },
  ],
  onAnswer: fn(),
  onDiscuss: fn(),
};

const commandWaiting: DockQuestion = {
  id: "01M21BKW3M0045T2Q8C1ZC7Q1R:command",
  repository: "armada",
  job: "14",
  title: "Preserve job metadata during resource cleanup",
  label: "The drone wants to run a command it was not given",
  asked: <span className="mono">cargo nextest run -p store</span>,
  waiting: "1m",
  answers: [
    { id: "allow_for_job", label: "Allow for this job", consequence: "The drone runs it now, and this job can run it again without asking." },
    { id: "always_allow", label: "Always allow in this repository", consequence: "The drone runs it now, and every job against this repository can run it without asking." },
    { id: "reject", label: "Reject", consequence: "The drone is told no, and the command does not run." },
  ],
  onAnswer: fn(),
  onDiscuss: fn(),
};

const judgeRefusal: DockQuestion = {
  id: "01M21BKX8H006A9RWZ3T0NQ4KD:judge",
  repository: "shop-01",
  job: "3",
  title: "Checkout total ignores the discount code",
  label: "Judge refused a criterion and is asking you",
  asked: "Does the fix address the cause the note names?",
  detail: "A customer with a valid code is still charged the full price.",
  waiting: "22m",
  answers: [
    { id: "agree", label: "Agree with the refusal", consequence: "The step fails, as it would where the criterion is marked refuse." },
    { id: "disagree_once", label: "Disagree, just this step", consequence: "The step advances. The next job is asked about this criterion again." },
    { id: "disagree_always", label: "Always disagree", consequence: "The step advances, and no later job in this repository is asked about this criterion." },
  ],
  onAnswer: fn(),
  onDiscuss: fn(),
};

/** A Drone's question offers its own two to four answers. */
export const ADroneQuestion: Story = { args: { questions: [droneQuestion] } };

/** A held command offers the three Armada answers Fleet sent. */
export const ACommandWaiting: Story = { args: { questions: [commandWaiting] } };

/** A Judge refusal offers agree, disagree once and disagree always. */
export const AJudgeRefusal: Story = { args: { questions: [judgeRefusal] } };

/**
 * Two repositories, each Job number meaning something only beside its repository. Pressing an
 * answer sends its wire name, and Discuss with Helm names the card it was pressed on.
 */
export const FromTwoRepositories: Story = {
  args: { questions: [judgeRefusal, droneQuestion, commandWaiting] },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getAllByRole("article")).toHaveLength(3);
    const refusal = canvas.getByRole("article", { name: /shop-01, job 3/ });
    await expect(refusal).toBeVisible();
    await expect(canvas.getByRole("article", { name: /armada, job 12/ })).toBeVisible();

    await userEvent.click(canvas.getByRole("button", { name: "Disagree, just this step" }));
    await expect(args.questions[0]?.onAnswer).toHaveBeenCalledWith("disagree_once");
    await expect(args.questions[2]?.onAnswer).not.toHaveBeenCalled();

    await userEvent.click(canvas.getAllByRole("button", { name: "Discuss with Helm" })[1]!);
    await expect(args.questions[1]?.onDiscuss).toHaveBeenCalledTimes(1);
    await expect(args.questions[0]?.onDiscuss).not.toHaveBeenCalled();
  },
};

/** Pressing an answer sends it — the whole of what a card does when Fleet takes it. #936. */
export const CardAnswered: Story = {
  args: { questions: [droneQuestion] },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Only running" }));
    await expect(args.questions[0]?.onAnswer).toHaveBeenCalledWith("Only running");
  },
};

/**
 * Stands in for the dock: a press goes out and `answering` follows on the
 * next render, the way `commands.acting` actually arrives.
 */
function Pressing() {
  const [answering, setAnswering] = useState(false);
  return (
    <DockQuestions
      questions={[
        {
          ...droneQuestion,
          answering,
          onAnswer: (answer) => {
            droneQuestion.onAnswer?.(answer);
            setAnswering(true);
          },
        },
      ]}
    />
  );
}

/**
 * Pressed, and Fleet has not answered. The pressed answer waits and the rest
 * of the group is off — `answering` alone cannot say which one was pressed,
 * so the card remembers it locally. #1117.
 */
export const CardPending: Story = {
  render: () => <Pressing />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Only running" }));
    await expect(canvas.getByRole("button", { name: "Only running" })).toHaveAttribute(
      "aria-busy",
      "true",
    );
    await expect(canvas.getByRole("button", { name: "Include exited" })).toBeDisabled();
  },
};

/**
 * Reloaded mid-answer: the card mounts already `answering`, with no local
 * press to mark. Every answer is off and none carries the bar — the block
 * has nothing of its own to point at. #1117.
 */
export const CardAnsweringElsewhere: Story = {
  args: { questions: [{ ...droneQuestion, answering: true }] },
  play: async ({ canvas }) => {
    const only = canvas.getByRole("button", { name: "Only running" });
    await expect(only).toBeDisabled();
    await expect(only).not.toHaveAttribute("aria-busy", "true");
    await expect(canvas.getByRole("button", { name: "Include exited" })).toBeDisabled();
  },
};

/** A window left open across an answer: the card says so, in Fleet's own words, and stays up. #936. */
export const AlreadyAnswered: Story = {
  args: {
    questions: [
      {
        ...commandWaiting,
        refusal:
          "the question being answered on job 01M21BKW3M0045T2Q8C1ZC7Q1R is not the one outstanding. " +
          "Read the job again and answer the one it names",
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent(/not the one outstanding/);
  },
};

/** A Judge card naming a question this job has since cleared, refused rather than applied to the one open now. #936. */
export const JudgeSuperseded: Story = {
  args: {
    questions: [
      {
        ...judgeRefusal,
        refusal: "the judge question being answered is not the one open now. Read the job again and answer the one it names",
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent(/not the one open now/);
  },
};

/** Before answering from the dock is wired: the answers are drawn, off, and the card says where to answer. */
export const AnswersNotWiredYet: Story = {
  args: {
    questions: [
      { ...droneQuestion, onAnswer: undefined, onDiscuss: undefined, note: "Open job 12 to answer." },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Only running" })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Discuss with Helm" })).toBeDisabled();
    await expect(canvas.getByRole("note")).toHaveTextContent("Open job 12 to answer.");
  },
};

/**
 * A call helm was held on, which has no job — so the card names its repository alone, says which
 * rule would have to allow it, and offers three answers of its own. #1389.
 */
export const HelmNeedsPermission: Story = {
  args: {
    questions: [
      {
        id: "helm:helm-1",
        repository: "armada",
        label: "Helm needs your permission",
        asked: <span className="mono">gh issue list --milestone Helm</span>,
        detail: "Your settings would have to allow Bash(gh issue list:*).",
        waiting: "18s",
        answers: [
          { id: "allow_once", label: "Allow once", consequence: "Helm runs it now. Nothing is written down." },
          {
            id: "allow_and_remember",
            label: "Allow and remember",
            consequence: "Helm runs it now, and the rule goes into this repository's own settings.",
          },
          { id: "refuse", label: "Refuse", consequence: "Helm is told no, and says what it could not do." },
        ],
        onAnswer: () => {},
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/armada · helm/)).toBeInTheDocument();
    await expect(canvas.queryByText(/job /)).not.toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Allow and remember" })).toBeEnabled();
  },
};
