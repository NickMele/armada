import { Fragment, useEffect, useState, type ReactNode } from "react";

import { Button, STILL_WAITING, useStillWaiting } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Every question waiting on a person, from every repository, in Helm's dock (#935). A card each.
 *
 * **The card names where it came from before what it asks**: on All repositories two Jobs share a
 * number, so a question without its repository is one a person cannot place.
 *
 * **No primary.** A list of decisions each taking the accent would be the fourteen accent blocks
 * the Button contract forbids a list; every answer is a secondary, and what it commits to is its
 * tooltip.
 */
export type DockQuestionsProps = {
  /** Oldest first. Empty draws nothing, and the dock draws its own quiet line. */
  questions: readonly DockQuestion[];
};

/** One answer a card offers. */
export type DockAnswer = {
  /** What the answer names on the wire — a Drone's own label, or `allow_for_job`, `agree`. */
  id: string;
  /** The words on the control. */
  label: string;
  /** What pressing it commits to. Absent draws no tooltip. */
  consequence?: string;
};

export type DockQuestion = {
  /** Unique across the list: a Job can hold a question and a command at once. */
  id: string;
  /** The picker's own label for the repository. */
  repository: string;
  /** The Job's number, `12`. */
  job: string;
  /** What the Job is called. */
  title?: string;
  /** The line over what was asked, naming the kind. */
  label: string;
  /** What was asked. A node, so a command sits in mono inside its sentence. */
  asked: ReactNode;
  /** A line under it — for a Judge refusal, what the difference does. */
  detail?: ReactNode;
  /** How long it has waited, already rendered. */
  waiting?: string;
  /** In the order to draw them. */
  answers: readonly DockAnswer[];
  /** Send one answer, by its `id`. **Absent draws the answers off**, and `note` says why. */
  onAnswer?: (answer: string) => void;
  /** True while this card's own answer is on its way to Fleet. Draws the answers off, no `note`. */
  answering?: boolean;
  /** Why the answers are off, where `onAnswer` is absent. */
  note?: ReactNode;
  /** Why this card's last answer did not take. Present beside live answers, not only where they are off. */
  refusal?: ReactNode;
  /** Point Helm at this card's repository and Job. The picker does not move. Absent draws it off. */
  onDiscuss?: () => void;
};

export function DockQuestions({ questions }: DockQuestionsProps) {
  if (questions.length === 0) return null;
  return (
    <section className="armada-dock-questions" aria-label="Waiting on you">
      <ul className="armada-dock-questions__list">
        {questions.map((question) => (
          <li key={question.id}>
            <QuestionCard {...question} />
          </li>
        ))}
      </ul>
    </section>
  );
}

function QuestionCard({
  repository,
  job,
  title,
  label,
  asked,
  detail,
  waiting,
  answers,
  onAnswer,
  answering,
  note,
  refusal,
  onDiscuss,
}: DockQuestion) {
  const where = `${repository}, job ${job}`;
  // Which of this card's own answers was pressed — `answering` alone says
  // Fleet has not answered, not which control sent it. Cleared once
  // `answering` drops, so a fresh question always starts unpressed. #1117.
  const [pressedId, setPressedId] = useState<string | null>(null);
  useEffect(() => {
    if (answering !== true) setPressedId(null);
  }, [answering]);
  const stillWaiting = useStillWaiting(answering === true);
  // `flat` because the questions zone is a well on the dock's own glass, and a
  // well is the one ground the card treatment's ancestry cannot see (#1353).
  // Inside the dock the rule reaches this card anyway; saying it here is what
  // makes it draw the same off the dock, in its own story.
  return (
    <Card className="armada-dock-question" flat role="article" aria-label={`${label} — ${where}`}>
      <div className="armada-dock-question__where">
        <span className="armada-dock-question__place mono">
          {repository} · job {job}
        </span>
        {waiting === undefined ? null : (
          <span className="armada-dock-question__waiting mono">{waiting}</span>
        )}
      </div>
      {title === undefined ? null : <p className="armada-dock-question__title">{title}</p>}
      <span className="armada-dock-question__label">{label}</span>
      <p className="armada-dock-question__asked">{asked}</p>
      {detail === undefined ? null : <p className="armada-dock-question__detail">{detail}</p>}
      <div className="armada-dock-question__answers" role="group" aria-label="Answers">
        {answers.map((answer) => {
          const pending = answering === true && pressedId === answer.id;
          const control = (
            <Button
              variant="secondary"
              size="sm"
              pending={pending}
              disabled={onAnswer === undefined || answering === true}
              onClick={() => {
                setPressedId(answer.id);
                onAnswer?.(answer.id);
              }}
            >
              {answer.label}
            </Button>
          );
          return answer.consequence === undefined ? (
            <Fragment key={answer.id}>{control}</Fragment>
          ) : (
            <Tooltip key={answer.id} label={answer.consequence}>
              {control}
            </Tooltip>
          );
        })}
      </div>
      {onAnswer === undefined && note !== undefined ? (
        <p className="armada-dock-question__detail" role="note">
          {note}
        </p>
      ) : null}
      {refusal === undefined ? null : (
        <p className="armada-dock-question__detail" role="alert">
          {refusal}
        </p>
      )}
      {stillWaiting ? (
        <p className="armada-dock-question__detail" role="status">
          {STILL_WAITING}
        </p>
      ) : null}
      <div>
        <Button variant="ghost" size="sm" disabled={onDiscuss === undefined} onClick={onDiscuss}>
          Discuss with Helm
        </Button>
      </div>
    </Card>
  );
}
