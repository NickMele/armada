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
};
