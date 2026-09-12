import { JobDetail, type JobDetailProps } from "../../../JobDetail";
import type { JobFixture } from "../../../fixtures/fixture";
import { propsFor } from "../../../fixtures/props";

/**
 * The app's job-detail screen, given one Job at one moment in the shape Fleet sends it.
 *
 * **Not a drawing of the screen — the screen.** Bridge renders `JobDetail`
 * from `JobSummary`, `Watched` and the reads beside them, deriving every
 * region itself. Stories used to hand-build that derivation's output and
 * could not show a bug in it — one shipped that way. This renders the same
 * component from the same inputs, mounted the way Bridge mounts it: a flex
 * column inside `.armada-screen__mounted`'s window-high content region,
 * filling it rather than growing past it. The first frame here borrowed
 * the component library's 400px stage and drew the screen cut off below it
 * — a story that fits a drawing is not the app.
 */
export function JobDetailFrom({
  fixture,
  width,
  on,
}: {
  fixture: JobFixture;
  /**
   * The handlers a story listens on, in place of the no-ops. **Only for a
   * `play` asserting what a press sends** — every other story draws a moment,
   * and a press there has nowhere to go.
   */
  on?: Partial<JobDetailProps>;
  /**
   * The window's width, where a story is about a narrow one — `--window-floor`
   * is the narrowest Bridge lays out for. Absent fills the story's own width.
   * It narrows the mount and not the window, so anything that follows the
   * window's own width, like a sheet's compact header, stays wide.
   */
  width?: string;
}) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        width,
        background: "var(--bg-base)",
      }}
    >
      <div className="armada-screen__mounted">
        <JobDetail {...propsFor(fixture)} {...on} />
      </div>
    </div>
  );
}
