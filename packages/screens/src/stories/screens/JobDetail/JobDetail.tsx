import { JobDetail } from "../../../JobDetail";
import type { JobFixture } from "../../../fixtures/fixture";
import { propsFor } from "../../../fixtures/props";

/**
 * The app's job-detail screen, given one Job at one moment in the shape Fleet
 * sends it.
 *
 * **Not a drawing of the screen — the screen.** Bridge renders `JobDetail` from
 * `JobSummary`, `Watched` and the reads beside them, and derives every region
 * itself. The stories for this screen used to hand-build what that derivation
 * produces, so they could not show a bug in it, and one shipped that way. This
 * renders the same component from the same inputs.
 *
 * **Mounted the way Bridge mounts it.** In the app the screen sits in
 * `.armada-screen__mounted`, a flex column inside a window-high content region,
 * and fills what that gives it rather than growing past it. So this is a
 * viewport-high column holding the same mount. The first frame here borrowed
 * the component library's 400px stage and drew the screen cut off below it —
 * a story that fits a drawing is not the app, and the app is what this shows.
 */
export function JobDetailFrom({ fixture }: { fixture: JobFixture }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        background: "var(--bg-base)",
      }}
    >
      <div className="armada-screen__mounted">
        <JobDetail {...propsFor(fixture)} />
      </div>
    </div>
  );
}
