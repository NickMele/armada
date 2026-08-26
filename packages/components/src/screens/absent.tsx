/**
 * A region of a screen whose component is not built.
 *
 * **This is scaffolding, not a component.** It has no registry row in
 * `components.toml`, no story of its own, and nothing outside `src/screens`
 * imports it. It exists so that a screen can be assembled from what is built
 * and still say, in the place where the missing thing goes, what is missing.
 *
 * The alternative was drawing the missing component here on the spot, which is
 * inventing one, or leaving the region blank, which reads as a screen that is
 * finished.
 *
 * **No M1 screen renders one now.** The four regions that carried it are built,
 * and the fifth — `Approval card` — is not held open, because a milestone that
 * does not render a thing should not draw a hole where it would go. This stays
 * for the next screen whose component is agreed and not yet built.
 */
export function Absent({ name, note }: { name: string; note: string }) {
  return (
    <div className="armada-screen-absent" role="note">
      <span className="armada-screen-absent__name">{name}</span>
      <span className="armada-screen-absent__why">{note}</span>
    </div>
  );
}
