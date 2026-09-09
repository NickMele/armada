// What a step's gates answered, as TypeScript sees it. `crates/ipc/src/judged.rs`.
//
// **Split out of `protocol.ts`, and the cut is one the Rust side already made.**
// That file reached the 900 lines the gate refuses for the fourth time, and
// `judged.rs` was itself split off `detail.rs` on the same line for the same
// reason: these are what the gates *said* about a step, and a Job's shape is a
// different sentence. Splitting anywhere else would have invented a seam;
// `protocol.ts` re-exports every name here, so nothing that imported one had to
// change.
//
// The header rules there hold here: these are hand-written, they drift the day
// a field moves, and every closed set is left as `string`.

/**
 * One criterion the Judge answered.
 *
 * **A refusal is not a failed Check and does not read as one.** A Check says
 * the work is broken; a refusal says the work runs and is not what was asked
 * for, which is why one ends the Job and the other escalates it. The three
 * optional fields are what a refusal owes and a no-objection does not: there is
 * nothing to cite where nothing was refused, and `""` would read as a citation
 * somebody lost.
 */
export type Judged = {
  /** Which run of the step this was answered on, counted from one. Since
   * 7.0. Joins to `StepDetail.attempts`. */
  attempt: number;
  /** Which criterion was asked. Joins to `JobDetail.acceptance_criteria`. */
  criterion_id: string;
  /**
   * Which member of the panel answered, counted from one. Since 7.7.
   *
   * **Absent at `panel_size: 1`**, the convention `DeclaredJudge.panel_size`
   * already keeps: a value always means a panel.
   *
   * **This is the only field that varies between the members of one panel.**
   * `attempt` and `criterion_id` are identical across them, so a row list keys
   * on this or it cannot key at all.
   *
   * A position, not a person, and never part of a citation.
   */
  member?: number;
  /** `criterion_verdict_judge`: `met` or `not_met`. */
  verdict: string;
  /** What should be seen if the work were right. */
  expected?: string;
  /** What is seen instead. */
  produced?: string;
  /** What that difference does to whoever consumes it. The triage line. */
  consequence?: string;
  /**
   * Where the whole brief this verdict answers was written, relative to the
   * repository root. **The path, never the question** — a brief is the request,
   * the deliverable and the whole branch diff, and no answer on this seam
   * carries one. Absent where Fleet kept no brief, which is a verdict nobody
   * can re-read against its input. Opened the way `CheckRun.output_path` is.
   */
  brief_path?: string;
  /**
   * Where in the brief this member's own words are quoted from. Since 8.3.
   *
   * **What one member read, which the verdict does not say.** The marks say
   * which way each judge went and the three fields say why; this says what each
   * of them was looking at, which is what separates two members refusing off
   * one line from two refusing off different ones.
   *
   * **Absent is ordinary and is not a gap.** A `met` answer writes no prose and
   * quotes nothing, and a refusal that describes rather than quotes has cited
   * in words nothing can place.
   */
  cited?: Citation[];
  /**
   * What this member's call was handed. Since 8.3.
   *
   * **The evidence that a panel was a panel.** Unanimity rests on the members
   * running against identical inputs, and until this the guarantee could not be
   * asked about on this seam: three rows carried three identical keys, and the
   * claim rested on the shape of a loop nobody outside Fleet can see. Compare
   * `digest` across the members — that is what the field is for.
   *
   * **Absent means nobody wrote it down**, which is every row a Fleet before
   * 8.3 produced. Never "the input was empty".
   */
  given?: Given;
};

/**
 * One quotation a verdict made, placed in the brief the call was shown.
 * `crates/ipc/src/judged.rs`.
 *
 * **Where the words are, never the words.** `Judged.brief_path` names the file
 * and this is a coordinate into it — so this is opened the way every kept
 * record is, through main, and the renderer never composes a path.
 *
 * **Not `Flagged.at`, which is the same question about the patch.** A gaming
 * flag points into the change; a Judge's citation points into what it was
 * shown. Two documents, and drawing one as the other sends a reader to a file
 * over a brief.
 */
export type Citation = {
  /**
   * Which labelled part of the brief holds it — `request`, `checks`,
   * `check:test_suite`, `reference:root_cause`, `deliverable`, `summary`,
   * `diff`, or `brief` for a line under no part.
   *
   * **An open set, and it is rendered rather than matched on.** Half of it is
   * named after a Check or a step the workflow declared, so nothing could hold
   * the list — `string`, like every closed set here and for a stronger reason.
   */
  region: string;
  /** The first line of the brief the quotation is on, counted from one. */
  from_line: number;
  /** The last. Equal to `from_line` where it does not cross a line break. */
  to_line: number;
};

/**
 * What one member of a panel was handed. `crates/ipc/src/judged.rs`.
 *
 * **Three readings of one object, because one is not enough to argue with.**
 * The digest says two members got the same thing or did not and says nothing
 * about what the thing was; the size and the model are what a person reads once
 * the answer is no.
 */
export type Given = {
  /**
   * A digest over the exact text this member's call was sent.
   *
   * **A comparison, never a signature.** The rows of one panel were written by
   * one build in one pass, which is the whole span it is compared across.
   */
  digest: string;
  /** How long that text was, in characters. */
  size: number;
  /** The model this member's call ran on. */
  model: string;
};

/**
 * One copy of a step's deliverable, as Fleet kept it.
 *
 * **A reference, never the document.** A deliverable is up to 16 KiB of text
 * and a detail is re-read every time an event names the open Job; the path is
 * what `main/open.ts` hands to the OS.
 *
 * **The attempt is on the row rather than implied by its position.** A step
 * worked three times keeps three copies and they are three different documents,
 * so a list a reader had to count through would make *the one the Judge read* a
 * guess. It is the same ordinal `StepAttempt.attempt` carries.
 */
export type KeptDeliverable = {
  /** Which run of the step wrote it, counted from one. Joins to `attempts`. */
  attempt: number;
  /**
   * Where the copy is, relative to the repository root.
   *
   * **Fleet checked it was there when it answered**, which is the one thing the
   * renderer cannot check for itself. It can still be gone by the time somebody
   * clicks it, and main says so.
   */
  path: string;
};

/**
 * One gaming pattern found, and what it was found in. **Never a verdict** — a
 * flag says the evidence is suspect, not that the step failed. `pattern` is a
 * string because no registry declares the set: it comes from what a workflow's
 * `flag_if` names.
 *
 * **A spelling and a quoted line are not a reason, and for six of the patterns
 * a reason was bought and thrown away.** Those six are decided by a model
 * against a question written to be argued with. Without the question and the
 * brief, a person reading `assertion_weakened` over a rustdoc sentence has to
 * re-derive the whole argument off the diff to find the flag was wrong — which
 * is the work the flag had already been paid for. `asked` and `brief_path` are
 * what it rests on.
 */
export type Flagged = {
  /** The pattern, spelled as `flag_if` spells it. */
  pattern: string;
  /** The file, line or assertion the flag is about. An uncited flag is unactionable. */
  cited: string;
  /**
   * Where in the change `cited` is, where Fleet established that from the
   * patch rather than asserting it. Since 6.2.
   *
   * **Absent is a real answer and never a gap to be filled.** A finding about
   * an absence has nothing to point at, and a citation quoting a line the
   * change removed has no post-image number. Absent draws no location rather
   * than a plausible one, which would send a person to the wrong file
   * believing it.
   */
  at?: CitedAt;
  /**
   * The narrow question this flag answers, word for word as the pattern put
   * it. Since 9.3.
   *
   * **What separates a real finding from a wrong one.** The spelling says what
   * shape of gaming was looked for and the citation says what was seen;
   * neither says what was claimed, and the claim is where the clauses that
   * decide it live — *and is that assertion made nowhere else in this change*
   * is the half nobody can reconstruct from the word `assertion_weakened`.
   *
   * **Absent is an answer about the check and not a gap.** `test_skipped`,
   * `test_deleted` and `check_config_edited` are decided by reading the diff,
   * so nothing was asked and there is no question to quote. A flag recorded
   * before Fleet kept this has none either.
   */
  asked?: string;
  /**
   * Where the whole brief this flag answers was written, relative to the
   * repository root. Since 9.3.
   *
   * **The path, never the question** — `Judged.brief_path`'s rule, and the
   * same opening: a brief is the request and the whole change, and no answer
   * on this seam carries one. It is kept under `.armada/briefs/` beside the
   * criteria briefs from the same step and attempt, so a flag and a verdict
   * from one run are read against each other.
   *
   * Absent wherever `asked` is, and additionally where the write itself
   * failed — a flag is not lost because a disk was.
   */
  brief_path?: string;
};

/**
 * Where in the change a flag points. `crates/ipc/src/judged.rs`.
 *
 * **`line` is a post-image coordinate**, numbered as this change leaves the
 * file. So a citation quoting a line the change removed carries the file and
 * no line: those words are not in that file any more, and where they used to
 * be now holds something else.
 *
 * **Not `Citation`, which is the same question about the brief.** That one is
 * a coordinate into what a Judge was shown; this is a coordinate into the
 * change, and drawing one as the other sends a reader to the wrong document.
 */
export type CitedAt = {
  /** Repository-relative, as the patch's post-image side spells it. */
  file: string;
  /** Absent where there is no post-image line, which is ordinary. */
  line?: number;
};
