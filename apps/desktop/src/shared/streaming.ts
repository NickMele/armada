// Where a recording plays from: the scheme, and what an address on it names.

// **A frame reaches the renderer two ways, and this is the second.** An image,
// a log or a JSON result crosses the preload as bytes and becomes a `blob:` —
// `frameOf` in main, unchanged. A recording cannot: a two-minute capture would
// have to be held whole before a frame of it drew, which is why the screen
// skipped one over twenty mebibytes and why a smaller one downloaded first.

// **Here rather than in `main/`, because the preload composes one of these.**
// The three bundles are built separately on purpose, so a preload that reached
// into main would blur exactly the line `contextIsolation` holds. What is
// shared is the spelling of an address; the forwarding is main's alone.

/**
 * The scheme a recording is drawn from.
 *
 * **Privileged, and admitted by the policy by name.** `index.html` widens
 * `media-src` to this and still refuses `http://127.0.0.1:*` — the renderer
 * never reaches Fleet's port, so what a page holds is a name main resolves.
 */
export const FRAME_SCHEME = "armada-frame";

/**
 * Where one frame's bytes stream from.
 *
 * **Composed from the id the record already gave out** — the run's directory
 * and the harness's own file name — so nothing here invents a path. The host is
 * a constant rather than the Job, because a scheme with a standard form needs
 * one and the Job is what the first segment is for.
 */
export function frameStreamUrl(jobId: string, kept: string): string {
  const path = kept.split("/").map(encodeURIComponent).join("/");
  return `${FRAME_SCHEME}://frame/${encodeURIComponent(jobId)}/${path}`;
}

/** What an address on this scheme names: a Job, and a frame of it. */
export type AskedFor = { jobId: string; kept: string };

/**
 * What a URL on this scheme names, or `null` where it names nothing.
 *
 * **Three segments and no others.** A frame id is a run directory and a file
 * name, so anything longer or shorter is not one — and a segment that decodes
 * to a separator or a climb is refused here as well as by Fleet, which resolves
 * every name against the record before it opens a file. Two checks for one rule
 * is the cheap half of a rule whose expensive half is somebody else's.
 */
export function askedFor(url: string): AskedFor | null {
  let read: URL;
  try {
    read = new URL(url);
  } catch {
    return null;
  }
  if (read.protocol !== `${FRAME_SCHEME}:` || read.hostname !== "frame") return null;
  const parts = read.pathname.split("/").filter((part) => part.length > 0);
  if (parts.length !== 3) return null;
  const [jobId, run, name] = parts.map(decodeURIComponent);
  if (jobId === undefined || run === undefined || name === undefined) return null;
  const climbs = (part: string): boolean =>
    part === "" || part === "." || part === ".." || part.includes("/") || part.includes("\\");
  if (climbs(jobId) || climbs(run) || climbs(name)) return null;
  return { jobId, kept: `${run}/${name}` };
}
