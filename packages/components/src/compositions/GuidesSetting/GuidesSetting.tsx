import { Button } from "../../primitives/Button/Button";
import { Switch } from "../../primitives/Switch/Switch";
import { useGuidance } from "../../guidance";

/**
 * The switch the first card carried, findable afterwards.
 *
 * **The offer is made once, on the card that arrived uninvited, and it lives
 * here from then on.** Somebody who turned every guide off on that card and
 * changed their mind has one place to look, and it is where every other choice
 * about this machine is made.
 *
 * It reads the guidance context rather than taking the value as a prop: there
 * is one answer for the window, and a second copy of it in a screen's state is
 * a switch that can disagree with the cards.
 */
export type GuidesSettingProps = {
  /** Opens the catalogue. Navigation is the window's. Absent draws no control. */
  onReadGuides?: () => void;
};

export function GuidesSetting({ onReadGuides }: GuidesSettingProps) {
  const { off, onOff } = useGuidance();
  return (
    <div className="armada-guides-setting">
      <Switch
        checked={!off}
        onChange={(event) => onOff(!event.currentTarget.checked)}
        description="A card opens once for a piece you have not met, and never again for that one. The ? beside a piece opens its guide whatever this says."
      >
        Open a guide the first time I meet a piece
      </Switch>
      {onReadGuides === undefined ? null : (
        <div className="armada-guides-setting__go">
          <Button variant="secondary" size="sm" ground="card" onClick={onReadGuides}>
            Read all guides
          </Button>
        </div>
      )}
    </div>
  );
}
