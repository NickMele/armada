import { JobComposer, type JobComposerProps } from "../../compositions/JobComposer/JobComposer";

/**
 * Journey · Dispatch a Job. The composer, in the frame the screen gives it.
 *
 * **The full approval card is not drawn beside it, and that is the design.**
 * The drawing puts it there as the dimmed half of a two-up; the three glance
 * fields it exists for are a diff size, a job type and an estimated cost, none
 * of which Armada can measure before a Drone has run. A region held open for it
 * would be a hole for something the milestone does not render, so the screen
 * renders the composer alone rather than at half width beside a blank.
 *
 * **Approve lands the Job in `queued`, not `running`**, and **Cancel writes
 * `killed`** — a Job never dispatched was not stopped, it was abandoned.
 */
export type DispatchAJobFullWithTheM1SubsetMarkedProps = {
  /** The composer. Every field, control and glance value is its own. */
  composer: JobComposerProps;
};

export function DispatchAJobFullWithTheM1SubsetMarked({
  composer,
}: DispatchAJobFullWithTheM1SubsetMarkedProps) {
  return (
    <div className="armada-screen__row">
      <div className="armada-screen__col" data-width="card">
        <JobComposer {...composer} />
      </div>
    </div>
  );
}
