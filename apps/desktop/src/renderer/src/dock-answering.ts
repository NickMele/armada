// Answering a card in Helm's dock (#936). Each kind's answer calls the endpoint Job detail
// already sends to; this is the wiring, apart from `App.tsx` for the reason `commands.ts` is.

import { useState } from "react";
import type { CommandAnswer, JudgeAnswer, Outcome } from "@armada/protocol";
import type { DockActs, Outstanding } from "@armada/screens";
import { outstandingId, refusalWords } from "@armada/screens";

import type { useCommands } from "./commands";

/**
 * `onAnswer` sends by kind and keeps the last refusal a card's own answer drew, by the card's own
 * id — cleared on a fresh attempt. `answering` reads `commands.acting`, the one flag every other
 * answer on this window already sets, so a card and a Job detail band open on the same Job agree
 * about whether something is on its way to Fleet.
 */
export function useDockAnswering(commands: ReturnType<typeof useCommands>): DockActs {
  const [refusals, setRefusals] = useState<Record<string, string>>({});

  async function onAnswer(question: Outstanding, answer: string): Promise<void> {
    const id = outstandingId(question);
    setRefusals(({ [id]: _dropped, ...rest }) => rest);
    const outcome = await sent(commands, question, answer);
    if (!outcome.ok) setRefusals((was) => ({ ...was, [id]: refusalWords(outcome) }));
  }

  return {
    onAnswer,
    answering: (question) => commands.acting === question.job_id,
    refusalFor: (question) => refusals[outstandingId(question)],
  };
}

function sent(
  commands: ReturnType<typeof useCommands>,
  question: Outstanding,
  answer: string,
): Promise<Outcome> {
  switch (question.kind) {
    case "drone":
      return commands.answer(question.job_id, question.asking.question_id, answer);
    case "command":
      return commands.answerCommand(question.job_id, question.waiting.call, answer as CommandAnswer);
    case "judge":
      return commands.answerJudge(question.job_id, question.question.asked_at, answer as JudgeAnswer);
  }
}
