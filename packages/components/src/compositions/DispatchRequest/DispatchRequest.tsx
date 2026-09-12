import type { LucideIcon } from "lucide-react";
import type { ChangeEvent, ClipboardEvent, ReactNode } from "react";
import { useRef } from "react";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { MentionPopover, useMention } from "../../primitives/MentionPopover/MentionPopover";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { ErrorNotice } from "../../errors/ErrorNotice/ErrorNotice";
import type { DebugPayload } from "../../errors/ErrorNotice/ErrorNotice";
import { ACTION } from "../../actions";
import { JOB_STATUS } from "../../generated/vocabulary";
import type { StagedAttachment } from "@armada/protocol";

/**
 * Dispatch a job by describing the work. One field, one press, and the Job
 * proposer decides the rest.
 *
 * Describing the work is the path; the form behind `Enter by hand` is the
 * override. `docs/concepts/job-proposer.md` says why: doing it by hand
 * means knowing the workflow catalogue before you can ask for anything.
 */

/**
 * The wait says what the call is doing, and offers a way out. Fleet
 * publishes how far the call has reached and what may stop it, so the wait
 * draws that instead of a dead button with a present-participle label.
 *
 * It still does not fill the proposal in progressively, and must not look
 * as though it does: Jobs arrive whole, once, at the end. What moves is the
 * call's own progress — reached the vendor, thinking, answering — a fact
 * about the wait, not a preview of the answer. A skeleton of Job rows would
 * claim rows are arriving one at a time, which is still not happening.
 */

/**
 * A wait needs more than an elapsed count: "ninety seconds and thinking" and
 * "ninety seconds and never reached the vendor" take opposite decisions and
 * look identical as a bare number. So the reach is drawn, and past
 * `slowAfterMs` the surface says so and offers the stop instead of waiting
 * for somebody to wonder.
 *
 * Stopping kills the call — it is not this window giving up. A wait
 * abandoned instead leaves the proposer running inside Fleet and spending,
 * with nobody left to read what it decides.
 */

/**
 * Two refusals, drawn as two different things. No workflow resolved is
 * Armada working: Fleet refused and returned the request unchanged, no red
 * and no code, and the two ways on are editing the request or entering the
 * job by hand.
 *
 * The call not being made at all is Armada failing: a fault, carrying the
 * code every error carries, rendered inline since a proposer that could not
 * be called stops this surface and nothing else. What to do about it is
 * Fleet's own sentence, since Fleet is what knows whether a budget ran out.
 */

/**
 * Nothing here decides scope, and nothing here may look like it did. A Job
 * reaches this gate with `write_targets` null, not empty — null is scope
 * not yet determined, empty would claim the Job writes nothing. So no path,
 * file count or diff estimate appears on a proposed job; the line under the
 * list says what the gate approves instead: the workflow, the name and the
 * split.
 */

/**
 * Approving happens here, on the job you just read. Every Job that comes
 * back already exists, at `awaiting_approval`. The head of the proposal
 * carries its own approval, since everything the gate approves — workflow,
 * name, split — is already on this screen. Settled 2026-09-08, against the
 * earlier reading that approval is always a second act from detail.
 *
 * It is one approval, and only the head's: Fleet's rule is strictly one by
 * one, and a chained Job is not at its gate until the one before it
 * completes, so every row under the first carries Review alone. The Job
 * Board is unchanged — a row somebody is browsing past still only
 * signposts, since the proposal is not on screen there. Review stays
 * beside the approval, for when the title is not enough.
 */
export type DispatchRequestProps = {
  /**
   * What is typed. Controlled by the caller, because a refusal hands the
   * request back unchanged and the field is where it comes back to.
   */
  request: string;
  onRequest: (request: string) => void;
  /**
   * Narrow the checkout against typed text, for the `@` mention popup —
   * `crate::files::search` on the other side of the wire. Never rejects: a
   * call that could not be made answers empty, the way `JobCommands.searchFiles`
   * does, so a popup nobody may even have open never raises a toast.
   */
  onSearchFiles: (query: string) => Promise<readonly string[]>;
  /**
   * Files pasted or picked against the request, before the Job it will
   * attach to exists. Controlled, the way `request` is — this draws the chips
   * and the picker, and the caller carries what comes back on `onDispatch`.
   */
  attachments: readonly StagedAttachment[];
  /**
   * Put a picked or pasted file somewhere the Job can name, and answer with
   * the path. Staged before any Job exists, so there is no id to key it on —
   * the same call `Composer` makes through `onStage`.
   */
  onStage: (bytes: ArrayBuffer, filename: string, mimeType: string) => Promise<{ path: string }>;
  /** One file staged, appended to `attachments`. */
  onAttach: (attachment: StagedAttachment) => void;
  /** Take a staged file back, by the path `onStage` answered with. */
  onRemoveAttachment: (path: string) => void;
  /** Send it. Never called with a blank request — the control is off until one. */
  onDispatch: () => void;
  /** Fill the form in by hand instead. The override, and one press away. */
  onEnterByHand: () => void;
  /** Drop what came back and describe something else. */
  onReset: () => void;
  /** Open one of the jobs that came back, where its own gate is drawn. */
  onOpen: (jobId: string) => void;
  /**
   * Release the job at the head of the proposal, which is what starts the work.
   *
   * **Offered on one row and never on two.** Only the head is at its gate — the
   * rest of a chain reach theirs as the one before them completes — so this is
   * called with the first job's id or not at all.
   */
  onApprove: (jobId: string) => void;
  /**
   * Jobs whose approval is out, by id. The control says `Approving` and goes
   * dead: approving twice does not spawn twice, but a control that looks
   * unpressed invites the second press and then says nothing about the first.
   */
  approving?: readonly string[];
  /** What the proposer answered, or that it has not been asked. */
  proposal: Proposal;
  /**
   * Stop the call that is out. **Kills it rather than stopping the wait** — a
   * wait abandoned leaves the proposer running inside Fleet and spending, with
   * nobody left to read what it decides.
   *
   * Absent where stopping is not offered, which draws no control rather than a
   * dead one.
   */
  onStop?: () => void;
  /**
   * How long a wait may run before the surface says so and puts the stop in
   * front of the person, in milliseconds.
   *
   * **A prompt, not a limit.** Nothing happens at this mark except that the
   * question is asked: the call keeps running until Fleet's own budget or until
   * somebody presses stop. It is the caller's because what counts as long is a
   * property of the deployment rather than of this component.
   */
  slowAfterMs?: number;
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
 * **Five states and no sixth.** There is no partial proposal: the call is asked
 * once and answers once. What `reading` gained is a description of the wait,
 * which is not a partial answer — see the type's own note.
 */
export type Proposal =
  /** Nothing asked. The ordinary opening state, and where a reset returns to. */
  | { at: "unasked" }
  /**
   * Asked, and waiting. **The proposal still arrives whole**; `watch` describes
   * the call, not the answer.
   *
   * Absent where Fleet has not said anything about the call yet, which is every
   * moment before the first event and every Fleet too old to send one. The
   * surface draws the wait without it rather than drawing nothing.
   */
  | { at: "reading"; watch?: ProposalWatch }
  /** Answered. Every job here exists already, at `awaiting_approval`. */
  | { at: "proposed"; request: string; jobs: readonly ProposedJob[] }
  /** No workflow resolved. The request is unchanged and no job was created. */
  | { at: "unresolved" }
  /** The call could not be made. A fault, and it carries a code. */
  | { at: "faulted"; code: string; message: ReactNode; payload?: DebugPayload };

/**
 * What the call is doing, while it does it.
 *
 * **Every number here is already resolved by the caller.** Elapsed is a
 * subtraction against a clock, and a component that read one would tick on its
 * own schedule and disagree with every other elapsed figure on screen.
 */
export type ProposalWatch = {
  /**
   * How far the call has got. `starting` is **the one worth telling apart**: a
   * call still there after a minute never reached the vendor at all, which will
   * not resolve by waiting.
   */
  reached: "starting" | "started" | "requesting" | "thinking" | "answering";
  /** How long the call has been out, in milliseconds. */
  elapsedMs: number;
  /** Fleet's own ceiling for this call, in milliseconds. */
  budgetMs: number;
  /** Which model is reading it. What the wait costs, roughly. */
  model: string;
  /**
   * The harness's running estimate of how much the model has thought. **Drawn
   * as an approximation**, because that is what it is.
   */
  thinkingTokens?: number;
  /** How much of the answer has arrived, in characters. */
  answeredCharacters?: number;
};

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

/**
 * What the row's forward control is called. The same row of `actions.toml` the
 * detail's own gate answers — one act, one word, wherever it is offered.
 */
const APPROVE = ACTION["approve"];

/** The status a job is at when its gate is somebody's to release. */
const AT_THE_GATE = "awaiting_approval";

/** What the field asks for, and the two things it takes. */
const PLACEHOLDER = "Describe the work, or paste a link to a ticket.";

/** Said on both refusals, because it is the fact a person most needs. */
const NOTHING_CREATED = "Nothing was created and the request is unchanged.";

export function DispatchRequest({
  request,
  onRequest,
  onSearchFiles,
  attachments,
  onStage,
  onAttach,
  onRemoveAttachment,
  onDispatch,
  onEnterByHand,
  onReset,
  onOpen,
  onApprove,
  approving = [],
  proposal,
  onStop,
  slowAfterMs,
  disabled = false,
  disabledNote,
  onCopied,
}: DispatchRequestProps) {
  const reading = proposal.at === "reading";
  const answered = proposal.at === "proposed";
  const empty = request.trim() === "";
  // The hidden file input the "Attach" button clicks through. A ref rather
  // than state because nothing here reads its value; `onChange` does.
  const fileInputRef = useRef<HTMLInputElement>(null);
  const mention = useMention(request, onRequest, onSearchFiles);

  /**
   * One file staged and handed to `onAttach`. Shared by the picker and a
   * pasted screenshot — both hand this the same three facts and differ only
   * in where the bytes came from. Mirrors `Composer`'s own `stage()`.
   */
  async function stage(file: File): Promise<void> {
    const bytes = await file.arrayBuffer();
    const { path } = await onStage(bytes, file.name, file.type);
    onAttach({ path, filename: file.name, mimeType: file.type });
  }

  function onFilesPicked(event: ChangeEvent<HTMLInputElement>): void {
    const files = event.target.files;
    if (files !== null) for (const file of Array.from(files)) void stage(file);
    // Cleared so picking the same file again still fires `onChange`.
    event.target.value = "";
  }

  /**
   * A screenshot pasted straight into the Request field, without a trip to
   * the file picker. `clipboardData.items` carries every kind a paste can
   * hold; only image entries are staged here, and plain text still falls
   * through to the field as text.
   */
  function onRequestPaste(event: ClipboardEvent<HTMLTextAreaElement>): void {
    for (const item of Array.from(event.clipboardData.items)) {
      if (!item.type.startsWith("image/")) continue;
      const file = item.getAsFile();
      if (file !== null) void stage(file);
    }
  }

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
            <Answered
              proposal={proposal}
              onOpen={onOpen}
              onApprove={onApprove}
              approving={approving}
            />
          ) : (
            <>
              <div className="armada-mention-anchor">
                <Textarea
                  label="Request"
                  rows={4}
                  value={request}
                  placeholder={PLACEHOLDER}
                  disabled={reading || disabled}
                  {...mention.fieldAria}
                  onChange={mention.onFieldChange}
                  onKeyDown={mention.onFieldKeyDown}
                  onSelect={mention.onFieldSelect}
                  onBlur={mention.onFieldBlur}
                  onPaste={onRequestPaste}
                />
                {/* The `@` mention popup, directly under the field it opened
                    on — see `MentionPopover`'s own note on why it is anchored
                    there and not at the caret. */}
                {mention.open ? (
                  <MentionPopover
                    query={mention.query}
                    results={mention.results}
                    active={mention.active}
                    listId={mention.listId}
                    optionId={mention.optionId}
                    onHover={mention.onHover}
                    onChoose={mention.onChoose}
                  />
                ) : null}
              </div>
              {/* Hidden behind the "Attach" button — no file input is ever
                  drawn directly, the platform's own picker chrome is not this
                  app's to style. */}
              <input
                ref={fileInputRef}
                type="file"
                multiple
                className="hidden"
                onChange={onFilesPicked}
              />
              <div>
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={reading || disabled}
                  onClick={() => fileInputRef.current?.click()}
                >
                  Attach
                </Button>
                {attachments.length > 0 && (
                  <div>
                    {attachments.map((attachment) => (
                      <AttachmentChip
                        key={attachment.path}
                        filename={attachment.filename}
                        onRemove={() => onRemoveAttachment(attachment.path)}
                      />
                    ))}
                  </div>
                )}
              </div>
            </>
          )}

          {/* The wait. The proposal still arrives whole; what moves here is
              the call's own progress, which is a fact about the wait rather
              than a preview of the answer. */}
          {proposal.at === "reading" ? (
            <Waiting
              {...(proposal.watch === undefined ? {} : { watch: proposal.watch })}
              {...(onStop === undefined ? {} : { onStop })}
              {...(slowAfterMs === undefined ? {} : { slowAfterMs })}
            />
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
 * The wait, and what to do about it.
 *
 * **Three registers, and which one is drawn turns on one thing**: whether the
 * wait has passed the mark where a person should be asked. Before it, the wait
 * is ordinary and says what the call is doing. After it, the surface says so
 * and puts the stop in front of them — rather than leaving somebody to wonder
 * whether anything is happening and find no way to end it.
 *
 * **Nothing here ticks.** Every figure is resolved by the caller against one
 * clock, so this and the rest of the window cannot disagree about how long a
 * thing has taken.
 */
function Waiting({
  watch,
  onStop,
  slowAfterMs,
}: {
  watch?: ProposalWatch;
  onStop?: () => void;
  slowAfterMs?: number;
}) {
  // No reading yet, and no mark to have passed. **The sentence that was here
  // before any of this**, kept for a Fleet that sends no progress and for the
  // moment before the first message lands.
  if (watch === undefined) {
    return (
      <p className="armada-dispatch__waiting" role="status">
        The proposer is reading the request. It answers once, whole.
      </p>
    );
  }

  const slow = slowAfterMs !== undefined && watch.elapsedMs >= slowAfterMs;
  const left = Math.max(0, watch.budgetMs - watch.elapsedMs);

  return (
    <div className="armada-dispatch__wait" role="status">
      <p className="armada-dispatch__wait-head">
        <span className="armada-dispatch__wait-what">{REACHED[watch.reached]}</span>
        <span className="armada-dispatch__wait-for">{lasting(watch.elapsedMs)}</span>
      </p>
      {/* The model and the ceiling on one line. The ceiling is what makes the
          elapsed figure mean anything: against nothing it can only say "slow",
          and against the budget it says how much of the decision is left. */}
      <p className="armada-dispatch__wait-where">
        {watch.model} · {left === 0 ? "out of time" : `${lasting(left)} left`}
      </p>
      {/* What it has actually done. Absent rather than zeroed: a call that has
          not started thinking and one thinking about nothing are different
          things, and a `0` would draw them the same. */}
      {watch.thinkingTokens === undefined ? null : (
        <p className="armada-dispatch__wait-count">
          about {watch.thinkingTokens.toLocaleString()} tokens of thinking
        </p>
      )}
      {watch.answeredCharacters === undefined ? null : (
        <p className="armada-dispatch__wait-count">
          {watch.answeredCharacters.toLocaleString()} characters of answer so far
        </p>
      )}
      {slow ? (
        <div className="armada-dispatch__wait-slow">
          <p className="armada-dispatch__wait-ask">
            This is taking longer than expected. It is still running — waiting is
            reasonable, and so is stopping.
          </p>
          {/* Only the stop. **There is no `Keep waiting` control**, and the
              absence is the design: waiting is what happens if nothing is
              pressed, and a button for it would be a control that performs no
              act — the one thing a surface must not offer. Dismissing the
              notice would be worse again, hiding the only way out of the wait.
              */}
          {onStop === undefined ? null : (
            <Button variant="secondary" onClick={onStop}>
              Stop the proposer
            </Button>
          )}
        </div>
      ) : null}
    </div>
  );
}

/**
 * What each reach is called on screen.
 *
 * **`starting` is the one that says something is wrong.** A call that has not
 * announced itself never reached the vendor, so its sentence names the harness
 * rather than the model — that is the reading a person needs in order to stop
 * rather than wait.
 */
const REACHED: Record<ProposalWatch["reached"], string> = {
  starting: "Starting the proposer",
  started: "Waiting to reach the model",
  requesting: "Asking the model",
  thinking: "The model is thinking",
  answering: "The answer is arriving",
};

/**
 * A duration, in the coarsest unit that is still true. Seconds under a minute,
 * then minutes and seconds — a wait is read at a glance, and `142s` is a number
 * somebody has to divide.
 */
function lasting(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return rest === 0 ? `${minutes}m` : `${minutes}m ${rest}s`;
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
  onApprove,
  approving,
}: {
  proposal: Extract<Proposal, { at: "proposed" }>;
  onOpen: (jobId: string) => void;
  onApprove: (jobId: string) => void;
  approving: readonly string[];
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

            <div className="armada-dispatch__job-acts">
              {/* Opens the job, for the case where the title is not enough. It
                  is not the way to approve one any more, but it is still the
                  only way to read one before releasing it. */}
              <Button
                variant="secondary"
                size="sm"
                onClick={() => onOpen(job.id)}
                aria-label={`${REVIEW === undefined ? "Review" : REVIEW.verb} ${job.title}`}
              >
                {REVIEW === undefined ? "Review" : REVIEW.verb}
              </Button>

              {/* The gate, on the one row that holds it. The status is Fleet's
                  and is checked rather than assumed: a job already released —
                  by this press or from anywhere else — draws no second one. */}
              {index === 0 && job.status === AT_THE_GATE ? (
                <Releasing
                  job={job}
                  out={approving.includes(job.id)}
                  onApprove={onApprove}
                />
              ) : null}
            </div>
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
 * The gate, as a control. **The label and the accessible name are the same
 * words** — a button reading `Approving` under a name saying `Approve` tells a
 * screen reader the press is still there to make.
 */
function Releasing({
  job,
  out,
  onApprove,
}: {
  job: ProposedJob;
  /** This job's approval is in flight. */
  out: boolean;
  onApprove: (jobId: string) => void;
}) {
  const verb = APPROVE === undefined ? "Approve" : APPROVE.verb;
  const says = out ? "Approving" : verb;
  return (
    <Button
      variant="primary"
      size="sm"
      disabled={out}
      onClick={() => onApprove(job.id)}
      aria-label={`${says} ${job.title}`}
    >
      {says}
    </Button>
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
