// Whether the repository still has what `armada.yml` names.
// `crates/ipc/src/drift.rs`.
//
// **A read of the repository, where `reading.ts` is a read of the file.** That
// one is what Fleet could not adopt when `armada.yml` changed; a `run` line
// naming a script somebody deleted parses perfectly, is adopted without
// complaint, and says nothing. The first thing that notices is a Job failing on
// it.
//
// **Free, and that is what makes it a read.** It is a handful of `stat` calls
// and runs on opening the surface, beside a dry-run that runs a real test suite
// behind its own button. Nothing on this seam ever runs a command to find out
// whether one exists.
//
// The header rules in `protocol.ts` hold here: these are hand-written, they
// drift the day a field moves, and every closed set is left as `string`.

/**
 * What the repository still has of what one Manifest names.
 *
 * **A clean list is not "all of this is still right", and a surface drawing
 * this has to say so.** About a third of a mature Manifest — policy,
 * permissions, budgets, ports — names nothing runnable and has no row here at
 * all, and most rows that do exist were checked against nothing. `checked` on
 * `current` is what lets the panel state which rows its clean list is about.
 *
 * It also says nothing about whether a listed command still does the right
 * thing: a `test` script narrowed to one directory reads as existing.
 */
export type ManifestDrift = {
  /** The file this was read against, as Fleet resolved it. */
  path: string;
  /**
   * The checkout the paths were looked for in. **The tree on disk**, not a
   * Job's worktree — drift answers about the repository a person is about to
   * run something in.
   */
  checkout: string;
  /**
   * One row per `run`, `serve` or `ready` line the file declares, in the order
   * `armada.yml` writes them.
   */
  declarations: Declaration[];
};

/**
 * One line the Manifest declares, and whether what it names is still there.
 *
 * **One row per line, not per declaration.** A Command that serves declares up
 * to three — what builds it, what starts it, and what says it is ready — and
 * each can go missing on its own, so one row carrying a verdict for all three
 * would be a verdict about none of them.
 */
export type Declaration = {
  /**
   * The section, spelled as `armada.yml` spells it — `checks`, `commands`.
   * Rendered, never matched on, so a registry added later draws as itself.
   */
  section: string;
  /** The name the file declares it under. */
  name: string;
  /**
   * The key inside it whose line this is — `run`, `serve`, `ready`. With the
   * two above it this is the line's path in `armada.yml`, which is what a
   * person searches the file for.
   */
  key: string;
  /**
   * The line, verbatim as the repository wrote it. `${port.NAME}` is
   * unresolved, because that is what the file says.
   */
  run: string;
  /** Whether the repository still has what it names. */
  drift: Drift;
  /**
   * What this line names that the read did not follow, and why, **whatever the
   * verdict**. Empty only where every word that could name something runnable
   * was resolved.
   *
   * **This is what stops a clean list overstating itself.** Drift follows
   * `pnpm` and `npm` into `package.json` and `cargo` into its aliases and
   * workspace; a tool it does not know lands here rather than reading as clean.
   * It is never a third verdict — it belongs on a `gone` row as much as on a
   * `current` one.
   */
  unfollowed: Unfollowed[];
};

/**
 * One word a line names that the drift read did not follow.
 *
 * **Never evidence of absence.** A script name resolved to a `package.json`
 * that could not be read lands here, not in `missing`: a false `gone` teaches
 * a person the amber means nothing.
 */
export type Unfollowed = {
  /** The word, as the line spells it once quotes are taken off. */
  word: string;
  /** Why it was not followed. Rendered, never matched on. */
  why: string;
};

/**
 * The verdict, carrying what it was reached on.
 *
 * **Two verdicts and there is no third.** Nothing here means *changed*: the
 * file carries no record of what it was written against, so calling a script
 * changed would need a stored scan of a previous one. Drift never reports a
 * script the repository picked up that the file does not yet name, either —
 * existence alone needs no history.
 *
 * **Amber throughout and never red.** A drifted file is behind, not broken; the
 * dry-run beside it is what would prove broken.
 */
export type Drift = DriftCurrent | DriftGone;

/** Every repository path this line names is still in the checkout. */
export type DriftCurrent = {
  verdict: "current";
  /**
   * How many things this line named that the read looked for and found — a
   * path, a `package.json` script, a cargo alias or workspace member. **Zero
   * is common and is not a clean bill**: read it beside `unfollowed`, which
   * says what was not looked for. A row drawn green off a `checked` of zero is
   * making a claim this read did not make.
   */
  checked: number;
};

/**
 * The line names repository paths that are not in the checkout.
 *
 * **Reported, never fixed.** There is no Apply and no Accept all; acting on a
 * row means going to Edit, where the consequence is stated.
 */
export type DriftGone = {
  verdict: "gone";
  /**
   * What is missing, in the order the line names it, each spelled to name the
   * file a person would open: a path (`scripts/lint.sh`), or a file and the key
   * it lacks (`packages/web/package.json: scripts.build`). Never empty — and
   * every one of them, because a person correcting a file from a message naming
   * one saves and meets the next.
   */
  missing: string[];
};
