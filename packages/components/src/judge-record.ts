/**
 * The Judge record's three fields, named once.
 *
 * `docs/contracts/agent-copy.md` specifies a refusal as a fixed set of
 * named fields rather than prose, and says why: "Generated text is
 * specified by what it must contain, never by what shape it takes. A
 * structural rule gets satisfied identically forever and produces
 * twenty interchangeable paragraphs." The labels are Fleet's fixed
 * copy, where uniformity is the point.
 */

/**
 * Here rather than in the component that first drew them: two surfaces
 * render this record — `CriterionVerdicts`, which draws one Judge's
 * answer beneath the step it judged, and `JudgeRefusal`, which draws a
 * panel's. Each had its own copy of the three names, and a record whose
 * field list lives in two places is a record that can be renamed in
 * one of them.
 *
 * The markup is not shared and does not need to be: the contract
 * governs which fields exist, what each holds and what they are
 * called; how a rail draws them against how a sheet does is a layout
 * question with two honest answers. What may not differ is the
 * vocabulary.
 */

/** The fields, in the order the contract lists them. */
export const JUDGE_FINDING = ["expected", "produced", "consequence"] as const;

export type JudgeFindingField = (typeof JUDGE_FINDING)[number];

/** Fleet's fixed labels. Sentence case, like every other label in the app. */
export const JUDGE_FINDING_LABEL: Record<JudgeFindingField, string> = {
  expected: "Expected",
  produced: "Produced",
  consequence: "Consequence",
};

/**
 * What each field holds, for a hover — the contract's own descriptions, and
 * the two rules a reader needs to judge whether a Judge obeyed them.
 *
 * `consequence` says it never reaches the Drone because that is the fact which
 * explains the other two: `expected` has to name the action and its result
 * together, since it is the field that carries the whole weight on a retry.
 */
export const JUDGE_FINDING_SAID: Record<JudgeFindingField, string> = {
  expected:
    "What should be seen if the work is right, as the value itself — and the action that produces it, because this is the field a Drone is given on a retry.",
  produced: "What is seen instead, as the same kind of value.",
  consequence:
    "What that difference does to whoever consumes it. The field a person triages on; it never reaches the Drone.",
};
