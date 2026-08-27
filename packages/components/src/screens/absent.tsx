/**
 * A region of a screen with nothing to draw in it.
 *
 * **This is scaffolding, not a component.** It has no story of its own and
 * nothing outside `src/screens` imports it. It exists so that a screen can be
 * assembled from what is built and still say, in the place where the missing
 * thing goes, what is missing.
 *
 * The alternative was drawing the missing thing here on the spot, which is
 * inventing one, or leaving the region blank, which reads as a screen that is
 * finished.
 *
 * **Two kinds of missing, one treatment.** It was written for a component that
 * is not built; the three job detail screens render it for a value Fleet does
 * not serve. Both are a hole in a finished-looking screen, and `note` is where
 * the screen says which one this is.
 */
export function Absent({ name, note }: { name: string; note: string }) {
  return (
    <div className="armada-screen-absent" role="note">
      <span className="armada-screen-absent__name">{name}</span>
      <span className="armada-screen-absent__why">{note}</span>
    </div>
  );
}
