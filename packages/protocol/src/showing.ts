/**
 * What a step's harness produced, as Bridge is told about it. Since 9.2.
 *
 * **The row rides the record and the bytes are fetched.** `KeptFrame` is the
 * cheap fact — that there is a frame, what it is called, what it weighs, where
 * it is — and it rides `StepDetail`, which is re-read on every event naming the
 * open Job. The image is fetched over HTTP, once, by whoever opens one. That is
 * the split `CheckOutput` and `CallArguments` were made on, against the same
 * measurement: a frame is hundreds of kilobytes and `/events` is one
 * drop-oldest channel carrying every Job, so a payload that size would evict
 * the state changes the Board is drawn from. **Nothing here is published, and
 * no event kind carries a frame.**
 *
 * The bytes route answers the file itself rather than JSON, because an image
 * has no window — a truncated PNG is not a shorter PNG, it is a file nothing
 * can draw. So there is no partial reading to describe and nothing an envelope
 * would carry but a base64 that inflates the bytes by a third.
 */

/**
 * Which checkout a frame is a photograph of. Since 9.5.
 *
 * **What makes a set of frames a before and an after.** #209 asks for the
 * branch *and* the base so a reviewer sees what changed, and two frames named
 * `home.png` are only a pair if something says which is which — nothing else
 * on `KeptFrame` does, since the name is the harness's own.
 *
 * **Pair them by `name`, and read the leftovers as answers, not gaps.** A name
 * on both sides is a before and after; only on the branch is a screen the
 * change *added*; only at base is one it *removed*. None is a fault — drawing
 * a missing half as an error would draw the commonest case #209 exists for, a
 * brand-new screen, as the feature being broken.
 */
export type Side = "base" | "branch";

/**
 * One frame a step's harness produced, as Fleet kept it.
 *
 * **A reference, never the image**, the way `KeptDeliverable` and
 * `CheckRun.output_path` are references, and for this module's reason.
 *
 * **No caption, and no account of what it shows.** A screenshot of the wrong
 * state looks exactly like one of the right state, and the only thing that
 * makes a frame checkable is the spec that produced it — which is code, in the
 * diff, next to the change. A sentence here would be the Drone attesting to its
 * own work in a field nothing can check.
 */
export type KeptFrame = {
  /**
   * Which run of the step produced it, counted from one. Joins to `attempts`.
   *
   * **On the row rather than implied by its position.** A step worked three
   * times captured three sets and they are three different screens; a list a
   * reader had to count through would make *the one that passed* a guess.
   */
  attempt: number;
  /**
   * What the harness called it — the file's own name in the directory
   * `evidence.frames` points at.
   *
   * **The spec's words, and the only words there are.** A person scanning a
   * step's frames reads these, so a harness that names them
   * `job-detail-refused.png` has said something and one that names them `1.png`
   * has not. That is the repository's choice to make, and Armada neither
   * renames nor supplies a default.
   */
  name: string;
  /**
   * Where the copy is, relative to the repository root.
   *
   * **Fleet checked it was there when the answer was built**, which is
   * `KeptDeliverable.path`'s property and the one thing no client can check for
   * itself. It can still be gone by the time somebody opens it.
   */
  path: string;
  /**
   * What the file weighs, in bytes. **The one number that decides whether to
   * ask for it**, and it is here rather than discovered by asking.
   */
  bytes: number;
  /**
   * What a caller names this frame by on the bytes route — the run's directory
   * and the file name, joined.
   *
   * **Sent back as it was given, never derived from `path`.** Fleet composes
   * it, so there is one spelling of the identity; a client that rebuilt it out
   * of the path would be a second one, and the two would disagree the first
   * time either side changed how a frame is stored.
   */
  kept: string;
  /**
   * Which checkout this one is a photograph of. Since 9.5.
   *
   * **Optional, and absent means `branch`.** Every row written before 9.5 was
   * taken on the Job's own worktree, because that was the only place a harness
   * ran; a Bridge talking to an older Fleet reads the absence, not a gap.
   */
  side?: Side;
  /**
   * A digest of this frame's own bytes. Since 9.6.
   *
   * **Lets a surface fold a pair away without fetching either image** — a
   * spec that photographs ten screens touches one, and comparing digests is
   * how the nine that did not change are cut, at no round trip.
   *
   * **Sound in one direction only: differ means *draw it*** (at worst noise,
   * a PNG encoder may spell one picture two ways); **agree means *fold it***,
   * checked beside `bytes` since being wrong there hides the change.
   *
   * **Empty or absent is not a match.** A frame kept before this field
   * existed carries none, and two of those must not read as a pair that
   * agrees — it is drawn, as it would have been anyway.
   */
  digest?: string;
};

/**
 * Whether a person can ask this Job to show its work again, and every time
 * somebody did. Since 10.1, on `JobDetail.show_again`.
 *
 * **Facts, not a verdict.** Each field is one thing Fleet checked, and the
 * control reads them in its own order to say why it cannot run — which is why
 * there is no closed set of reasons on the wire to match on.
 */
export type ShowAgain = {
  /** Whether `armada.yml` declares an `evidence:` harness. */
  harness: boolean;
  /** Whether the Job's worktree is on disk. A clean or a reclaim takes it. */
  worktree_on_disk: boolean;
  /**
   * The spec a press reruns — the last one a Drone named on a `shown` step.
   * **Absent where no Drone ever named one.**
   */
  spec?: NamedSpec;
  /** Whether a Drone is working in the worktree right now. */
  drone_working: boolean;
  /** When the press out right now began. Absent where none is. */
  showing_since?: string;
  /**
   * Every press this Job kept, oldest first. **Beside the step's own frames
   * and never in them** — `StepDetail.frames` stays what the step produced.
   */
  shown: ShownSet[];
};

/** The spec a press reruns, and the run of the step that named it. */
export type NamedSpec = {
  step_id: string;
  /** Which run of the step named it, counted from one. */
  attempt: number;
  /** The Drone's own `shown_by`, as it submitted it. */
  spec: string;
  /** Whether the spec is still in the worktree. A later run may have moved it. */
  on_disk: boolean;
};

/**
 * What one press captured. **Told apart by when it ran**: two presses are two
 * sets, and neither replaces the step's frames.
 */
export type ShownSet = {
  /** The Job's own count of presses, one-based. */
  press: number;
  pressed_at: string;
  /** The step whose spec was rerun, and the run of it that named the spec. */
  step_id: string;
  attempt: number;
  /** Never empty: a press that captured nothing kept no set. */
  frames: KeptFrame[];
};

/**
 * The answer to `POST /jobs/:job_id/show_again`: the set the press kept, or
 * why there is none. **Never both and never neither.** A harness that ran and
 * captured nothing is not a refusal, so it is a 200 carrying the sentence the
 * Job's own log carries.
 */
export type ShownAgain = {
  job_id: string;
  set?: ShownSet;
  nothing?: string;
};
