import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { ErrorNotice } from "../../errors/ErrorNotice/ErrorNotice";
import type { DebugPayload } from "../../errors/ErrorNotice/ErrorNotice";
import { ACTION } from "../../actions";
import { JOB_STATUS } from "../../generated/vocabulary";

/**
 * Dispatch a job by describing the work. One field, one press, and the Job
 * proposer decides the rest.
 *
 * **Describing the work is the path; the form is the override.** A person types
 * what they want done, or pastes a link, and dispatches. They pick no workflow
 * and write no title — `../../../../../docs/concepts/job-proposer.md` says
 * both, and says why: doing it by hand means knowing the workflow catalogue
 * before you can ask for anything. Hand entry is one control away and is the
 * exception.
 *
 * # It does not fill in progressively, and this does not pretend it does
 *
 * The policy describes the proposal "filling in as it is worked out". What
 * shipped is one request and one response, with no stream behind it, so the
 * wait here is Bridge's own built idiom for an act in flight: the control takes
 * a present-participle label and goes dead, exactly as `Approve dispatch`
 * becomes `Approving`. A skeleton would claim rows are arriving one at a time,
 * which is the thing that is not happening.
 *
 * # Two refusals, drawn as two different things
 *
 * **No workflow resolved is Armada working.** Fleet answered, refused, and
 * returned the request unchanged — no Job was created. It takes no red and no
 * code: it is the surface saying what it will not send, and the two ways on are
 * to edit the request or to enter the job by hand.
 *
 * **The call not being made at all is Armada failing.** That is a fault, it
 * carries the code every error carries, and it renders in the error treatment
 * inline — the placement blast radius picks, since a proposer that could not be
 * called stops this surface and nothing else. What to do about it is Fleet's
 * own sentence, because Fleet is what knows whether a budget ran out.
 *
 * # Nothing here decides scope, and nothing here may look like it did
 *
 * A Job reaches this gate with `write_targets` null — not empty. Null is scope
 * not yet determined; empty would claim the Job writes nothing. So no path, no
 * file count and no diff estimate appears on a proposed job, and the line under
 * the list says out loud what the gate approves: the workflow, the name and the
 * split.
 *
 * # Approving is somewhere else, and stays somewhere else
 *
 * Every Job that comes back already exists, at `awaiting_approval`. This does
 * not approve one, and it offers no control that approves several at once —
 * nothing on a list approves, approval is a second act from detail, and Fleet's
 * rule is strictly one by one. The control on a row is Review, which is the
 * same signpost the Job Board's own row carries for this status.
 */
export type DispatchRequestProps = {
  /**
   * What is typed. Controlled by the caller, because a refusal hands the
   * request back unchanged and the field is where it comes back to.
   */
  request: string;
  onRequest: (request: string) => void;
  /** Send it. Never called with a blank request — the control is off until one. */
  onDispatch: () => void;
  /** Fill the form in by hand instead. The override, and one press away. */
  onEnterByHand: () => void;
  /** Drop what came back and describe something else. */
  onReset: () => void;
  /** Open one of the jobs that came back, where its own gate is drawn. */
  onOpen: (jobId: string) => void;
  /** What the proposer answered, or that it has not been asked. */
  proposal: Proposal;
  /** Nothing may be dispatched while the connection is not live. */
  disabled?: boolean;
  /** Why the controls are off, where they are. A dead control with no reason reads as broken. */
  disabledNote?: ReactNode;
  /** What the surface is told after a clipboard write, so it can raise a toast. */
  onCopied?: (what: string) => void;
};

/**
 * Where the one call has got to.
 *
 * **Five states and no sixth.** There is no partial proposal, because there is
 * no stream: the call is asked once and answers once.
 */
export type Proposal =
  /** Nothing asked. The ordinary opening state, and where a reset returns to. */
  | { at: "unasked" }
  /** Asked, and waiting. Nothing arrives until all of it does. */
  | { at: "reading" }
  /** Answered. Every job here exists already, at `awaiting_approval`. */
  | { at: "proposed"; request: string; jobs: readonly ProposedJob[] }
  /** No workflow resolved. The request is unchanged and no job was created. */
  | { at: "unresolved" }
  /** The call could not be made. A fault, and it carries a code. */
  | { at: "faulted"; code: string; message: ReactNode; payload?: DebugPayload };

/** One job the request became. **No scope, because none was proposed.** */
export type ProposedJob = {
  id: string;
  /** What the proposer called it. Nobody typed this. */
  title: string;
  /**
   * The workflow's name, resolved by the caller. Never the id: an id in a
   * proposal is the one field a person cannot check.
   */
  workflow: string;
  /**
   * The job's own status off the wire, which at this gate is
   * `awaiting_approval`. **Carried rather than assumed** — the job exists
   * before this surface draws it, so the badge says what Fleet says.
   */
  status: string;
};

/**
 * A status as a badge draws it, from the generated vocabulary rather than
 * typed here — a second copy of a status word is a second vocabulary.
 *
 * `null` where the registry carries no verb, glyph or token for it, which
 * draws no badge rather than an invented one.
 */
function badgeOf(status: string): { status: string; icon: LucideIcon; verb: string } | null {
  const rendering = JOB_STATUS[status];
  if (rendering === undefined) return null;
  const { badgeStatus, icon, verb } = rendering;
  if (badgeStatus === null || icon === null || verb === null) return null;
  return { status: badgeStatus, icon, verb };
}

/**
 * What a row's control is called. `actions.toml` is the authority on the verb
 * and the binding, and `keys.ts` in `@armada/screens` reads the same row for
 * the Job Board's own `awaiting_approval` row — one act, one word.
 */
const REVIEW = ACTION["review"];

/** What the field asks for, and the two things it takes. */
const PLACEHOLDER = "Describe the work, or paste a link to a ticket.";

/** Said on both refusals, because it is the fact a person most needs. */
const NOTHING_CREATED = "Nothing was created and the request is unchanged.";

export function DispatchRequest({
  request,
  onRequest,
  onDispatch,
  onEnterByHand,
  onReset,
  onOpen,
  proposal,
  disabled = false,
  disabledNote,
  onCopied,
}: DispatchRequestProps) {
  const reading = proposal.at === "reading";
  const answered = proposal.at === "proposed";
  const empty = request.trim() === "";

  return (
    <Card className="armada-dispatch">
      <CardHeader>
        <CardTitle>Dispatch a job</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="armada-dispatch__body">
          {/* The whole reason this surface exists, said once. Three things the
              old form asked for are three things the proposer answers. */}
          <p className="armada-dispatch__lede">
            Armada reads the request, picks the workflow and names the job.
          </p>

          {answered ? (
            <Answered proposal={proposal} onOpen={onOpen} />
          ) : (
            <Textarea
              label="Request"
              rows={4}
              value={request}
              placeholder={PLACEHOLDER}
              disabled={reading || disabled}
              onChange={(event) => onRequest(event.target.value)}
            />
          )}

          {/* The wait, in the one register that is true: a call is out and
              nothing is arriving in pieces. */}
          {reading ? (
            <p className="armada-dispatch__waiting">
              The proposer is reading the request. It answers once, whole.
            </p>
          ) : null}

          {/* Refusal one. No red, no code — Fleet answered and declined, which
              is Armada working. The request is still in the field above. */}
          {proposal.at === "unresolved" ? (
            <div className="armada-dispatch__unresolved" role="status">
              <p className="armada-dispatch__unresolved-head">
                No workflow fits this request. {NOTHING_CREATED}
              </p>
              <p className="armada-dispatch__unresolved-body">
                Nothing is assigned by default: the workflow is frozen into the job at creation
                and becomes what the work is judged against. Edit the request and dispatch
                again, or enter the job by hand.
              </p>
            </div>
          ) : null}

          {/* Refusal two. Armada failing, so it takes the error treatment and
              its code. Inline, because a proposer that could not be called
              stops this surface and reaches nothing else. */}
          {proposal.at === "faulted" ? (
            <ErrorNotice
              kind="fault"
              placement="inline"
              code={proposal.code}
              message={
                <>
                  {proposal.message} {NOTHING_CREATED}
                </>
              }
              {...(proposal.payload === undefined ? {} : { payload: proposal.payload })}
              onCopied={onCopied}
              act={
                /* The act named, not repeated. The footer's own control is
                   `Dispatch again` while this is up, and a second button here
                   would be two controls for one act eight lines apart — which
                   is the thing the error treatment's "never a second decision"
                   rule is about. The sentence says the part the button cannot:
                   the fault said nothing about the request, so nothing has to
                   be edited before asking again. */
                <span className="armada-dispatch__act">
                  Dispatch again. The request is not what failed.
                </span>
              }
            />
          ) : null}

          {disabledNote === undefined ? null : (
            <p className="armada-dispatch__note">{disabledNote}</p>
          )}
        </div>
      </CardContent>

      <CardFooter className="armada-dispatch__foot">
        {answered ? (
          <Button variant="secondary" onClick={onReset}>
            Dispatch another
          </Button>
        ) : (
          <>
            {/* The override. Secondary and leading, because it is the exception
                and the accent is spent on the path. */}
            <Button variant="secondary" onClick={onEnterByHand} disabled={reading}>
              Enter by hand
            </Button>
            <Button
              variant="primary"
              onClick={onDispatch}
              disabled={reading || disabled || empty}
            >
              {reading
                ? "Reading the request"
                : proposal.at === "faulted"
                  ? "Dispatch again"
                  : "Dispatch"}
            </Button>
          </>
        )}
      </CardFooter>
    </Card>
  );
}

/**
 * What the request became.
 *
 * **The order is the whole of the graph.** A proposal of several is a chain —
 * each member waits on the one before it reaching `completed_success` — so
 * position carries it and no second field restates it.
 *
 * **One job draws no ordinal.** A list of one numbered `1` implies a second
 * that did not come.
 */
function Answered({
  proposal,
  onOpen,
}: {
  proposal: Extract<Proposal, { at: "proposed" }>;
  onOpen: (jobId: string) => void;
}) {
  const several = proposal.jobs.length > 1;

  return (
    <div className="armada-dispatch__answered">
      {/* What was asked, kept on screen. The proposal is only readable against
          the request it came from. */}
      <p className="armada-dispatch__asked">{proposal.request}</p>

      <p className="armada-dispatch__became">
        {several
          ? `The request became ${proposal.jobs.length} jobs, in this order.`
          : "The request became one job."}
      </p>

      <ol className="armada-dispatch__jobs">
        {proposal.jobs.map((job, index) => (
          <li className="armada-dispatch__job" key={job.id}>
            {several ? (
              <span className="armada-dispatch__ordinal mono" aria-hidden="true">
                {index + 1}
              </span>
            ) : null}
            <div className="armada-dispatch__job-body">
              <span className="armada-dispatch__title">{job.title}</span>
              <span className="armada-dispatch__workflow">{job.workflow}</span>
              {several && index > 0 ? (
                <span className="armada-dispatch__waits">{`Waits on job ${index}.`}</span>
              ) : null}
            </div>
            <AtTheGate status={job.status} />

            {/* Opens the job. It does not approve one: approval is a second act
                from detail, and a control here that dispatched would be a
                second gate over a proposal the first one already holds. */}
            <Button
              variant="secondary"
              size="sm"
              onClick={() => onOpen(job.id)}
              aria-label={`${REVIEW === undefined ? "Review" : REVIEW.verb} ${job.title}`}
            >
              {REVIEW === undefined ? "Review" : REVIEW.verb}
            </Button>
          </li>
        ))}
      </ol>

      {/* The two sentences this surface exists to prevent being guessed at. */}
      <p className="armada-dispatch__gate">
        {several
          ? "Each job is approved on its own, after the one before it completes. Nothing starts until you approve the first."
          : "Approving it is what starts the work."}
      </p>
      <p className="armada-dispatch__gate">
        The workflow, the name and the split are what you approve. No file is named yet — scope
        is the workflow&rsquo;s first step.
      </p>
    </div>
  );
}

/**
 * The badge on a proposed job. **The job's own status, not this surface's
 * idea of it** — every one of these exists on the board already, so a hardcoded
 * word here would be Bridge asserting something it was told.
 */
function AtTheGate({ status }: { status: string }) {
  const badge = badgeOf(status);
  if (badge === null) return null;
  return (
    <Badge status={badge.status} icon={badge.icon}>
      {badge.verb}
    </Badge>
  );
}
