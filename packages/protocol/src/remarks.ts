// What people wrote on a Job's pull request, as TypeScript sees it.
//
// **Hand-written, and a second statement of `crates/ipc/src/remarks.rs`** —
// `protocol.ts` says why that is a gap rather than a design, and this file has
// the same one.
//
// # The one place this seam carries text somebody outside this machine wrote
//
// Every other string in this package was written by a person at this keyboard,
// by a Drone Fleet spawned, or by Fleet itself. `Remark.said` was written by
// whoever can see the pull request, which on a public repository is anybody. It
// has not been trimmed, escaped or truncated anywhere between the forge and
// here: `adapter_traits::FromOutside` keeps it apart on the Rust side and it
// stops at the wire, because JSON has no other shape for a string.
//
// **So the guard on this side is where it is drawn.** React escapes a text node
// and this is only ever drawn as one — no `dangerouslySetInnerHTML`, no
// interpolation into a URL, no attribute. Nothing in Bridge decides anything
// from the content of one either: what a person does with a comment is pick it
// or not, and the only value that comes back is `id`.

/** Everything anybody has written on one Job's open pull request. */
export type JobRemarks = {
  job_id: string;
  /**
   * The address they were read off. **Named rather than assumed**, so a surface
   * can say which pull request it is showing without holding a second reading.
   */
  pull_request: string;
  /**
   * Oldest first, as the forge ordered them. **Empty is a pull request nobody
   * has commented on**, which is a real answer — a forge that would not answer
   * is a refusal and never this.
   */
  remarks: Remark[];
};

/** One comment on a pull request. `crates/ipc/src/remarks.rs`. */
export type Remark = {
  /**
   * What the forge calls it, and the only field that goes back on a press.
   * **Never drawn.** It identifies a comment across two reads of the same pull
   * request, which nothing else here can do: one person leaves two comments in
   * a minute, and an edited comment keeps its handle and changes its text.
   */
  id: string;
  /** The login of whoever wrote it, as the forge spells it. */
  by: string;
  /**
   * When they wrote it, as the forge wrote it. **A string and not an instant**
   * — parsing it would mint a clock value out of a remote's text, and nothing
   * orders these or measures anything from one.
   */
  at: string;
  /** What they wrote. */
  said: string;
  /**
   * Whether this comment has already been handed to a Drone on this Job.
   *
   * **Armada's own fact, and the one field here nothing outside wrote.** A
   * comment stays on a pull request forever and reads the same on every sweep,
   * so without this a comment already worked looks exactly like one nobody has
   * touched. Fleet refuses a press naming one, so a surface that offers it as
   * choosable is offering a press that will be refused.
   */
  taken_up: boolean;
};
