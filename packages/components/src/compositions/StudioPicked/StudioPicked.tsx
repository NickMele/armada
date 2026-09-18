import { Card, CardContent } from "../../primitives/Card/Card";
import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";

/**
 * What is picked on a Studio's whiteboard, and the one control holding every act
 * on it — #1399.
 *
 * **One control, not a row of buttons.** A row in an aside one node wide ran off
 * the surface at four acts and grows with every kind that arrives; a menu is the
 * shape that does not depend on how many there are, and `+ Node` is the same
 * control on the same surface.
 *
 * **Many picked says how many and names none.** Forty titles filled the window
 * and covered the zoom controls. Every rung's dialog still lists what it acts on.
 */

/** One act on what is picked. `id` is the caller's own word for it. */
export type StudioPickedAct = {
  id: string;
  /** Sentence case, naming what happens. */
  label: string;
  /** Deleting. Drawn last, under a separator, in the failure colour. */
  danger?: boolean;
};

export type StudioPickedProps = {
  /** Every picked node, named, in the order the board reports them. */
  picked: readonly string[];
  /** What the selection offers. None draws no control at all. */
  acts: readonly StudioPickedAct[];
  onAct: (id: string) => void;
  /** Something is already out to Fleet: the menu does not open a second one. */
  disabled?: boolean;
};

/**
 * The menu's entries, destructive acts last and under a separator — the rule
 * `DropdownMenu` states, kept here so no caller restates it.
 */
function entriesOf(acts: readonly StudioPickedAct[]): DropdownMenuEntry[] {
  const plain = acts.filter((act) => act.danger !== true);
  const danger = acts.filter((act) => act.danger === true);
  const item = (act: StudioPickedAct): DropdownMenuEntry => ({
    kind: "item",
    id: act.id,
    label: act.label,
    danger: act.danger,
  });
  const parted: DropdownMenuEntry[] =
    danger.length === 0 || plain.length === 0 ? [] : [{ kind: "separator", id: "before-danger" }];
  return [...plain.map(item), ...parted, ...danger.map(item)];
}

export function StudioPicked({ picked, acts, onAct, disabled = false }: StudioPickedProps) {
  if (picked.length === 0) return null;
  // One node is named. Several are counted: a Studio is picked over whole, and
  // forty titles is a panel taller than the window it is drawn in.
  const said = picked.length === 1 ? picked[0] : `${picked.length} nodes picked`;
  return (
    <Card role="group" aria-label="What is picked">
      <CardContent className="armada-studio-picked">
        <p className="armada-studio-picked__said">{said}</p>
        {acts.length === 0 ? null : (
          <DropdownMenu
            triggerLabel="Acts"
            disabled={disabled}
            entries={entriesOf(acts)}
            onSelect={onAct}
          />
        )}
      </CardContent>
    </Card>
  );
}
