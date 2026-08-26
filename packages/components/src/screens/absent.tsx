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
 * finished. Six names in the M1 drawing carry no story yet — `Board empty
 * state`, `Approval card`, `Job composer`, `Evidence card`, `Job detail header
 * actions` — and every one of them is a design conversation rather than a
 * `<div>`.
 */
export function Absent({ name, note }: { name: string; note: string }) {
  return (
    <div className="armada-screen-absent" role="note">
      <span className="armada-screen-absent__name">{name}</span>
      <span className="armada-screen-absent__why">{note}</span>
    </div>
  );
}
