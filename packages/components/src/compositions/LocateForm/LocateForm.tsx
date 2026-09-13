import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Input } from "../../primitives/Input/Input";
import { ScrollArea } from "../../primitives/ScrollArea/ScrollArea";
import { ErrorCode } from "../../errors/ErrorCode/ErrorCode";

/**
 * Locate — Journey 3's *Getting in*: add a repository by its folder, or clone one from a URL.
 *
 * **The folder is the act and the clone is the quieter one**, because most onboarding is of
 * code already on disk. **Controlled**: every value and what it came to arrive as props, so a
 * clone still running when the dialog closes is still running when it opens again.
 */

export type LocateMode = "folder" | "clone";

export type LocateFormProps = {
  open: boolean;
  mode: LocateMode;
  /** The folder to add. Mono, typed or chosen. */
  path: string;
  url: string;
  /** The folder a clone lands under. */
  parent: string;
  /** Where the clone lands, named as Fleet names it. `null` until the URL and the parent say. */
  landsIn: string | null;
  /** Whether the press is out. A clone can take minutes, and nothing is sent twice. */
  sending: boolean;
  /** Whether what is typed would be sent. */
  ready: boolean;
  /** A path that is not a full one, said under its own field. */
  pathMessage?: string;
  parentMessage?: string;
  /**
   * Fleet's refusal, in its own words — git's too — with its code, and what to do where the words
   * do not say. A failure that never reached Fleet has a sentence and no code.
   */
  refusal?: { code?: string; saying: string; next?: string };
  onMode: (mode: LocateMode) => void;
  onPath: (path: string) => void;
  onUrl: (url: string) => void;
  onParent: (parent: string) => void;
  /** The OS's folder dialog, for the field named. */
  onChoose: (field: "path" | "parent") => void;
  onSend: () => void;
  onCancel: () => void;
};

export function LocateForm(props: LocateFormProps) {
  const { open, mode, sending, ready, refusal, onMode, onSend, onCancel } = props;
  const cloning = mode === "clone";
  return (
    <Dialog
      open={open}
      tone="neutral"
      title="Add a repository"
      confirmLabel={cloning ? "Clone repository" : "Add repository"}
      confirmDisabled={!ready || sending}
      onConfirm={onSend}
      onCancel={onCancel}
      field={
        <div className="armada-locate">
          {cloning ? <CloneFields {...props} /> : <FolderFields {...props} />}
          {refusal === undefined ? null : (
            <div className="armada-locate__refusal" role="alert">
              {refusal.code === undefined ? null : <ErrorCode kind="fault" code={refusal.code} />}
              {/* Git's own words can run to pages; they scroll so the controls stay on screen. */}
              <ScrollArea className="armada-locate__said">
                <p className="armada-locate__saying">{refusal.saying}</p>
              </ScrollArea>
              {refusal.next === undefined ? null : <p className="armada-locate__next">{refusal.next}</p>}
            </div>
          )}
          {/* Gone while sending rather than disabled: the other act means nothing until this one answers. */}
          {sending ? null : (
            <div className="armada-locate__other">
              <Button variant="ghost" size="sm" onClick={() => onMode(cloning ? "folder" : "clone")}>
                {cloning ? "Add a folder on disk" : "Clone from a URL"}
              </Button>
            </div>
          )}
        </div>
      }
    >
      {sending && cloning ? (
        <p>
          Git is cloning into <span className="armada-locate__path">{props.landsIn}</span>. A large repository
          takes minutes. Closing this leaves the clone running, and it joins the project picker when it finishes.
        </p>
      ) : sending ? (
        <p>Fleet is reading the folder.</p>
      ) : cloning ? (
        <p>
          Git clones it into a new folder, with the credentials it already has. Armada serves the clone, and
          Setup opens for it next.
        </p>
      ) : (
        <p>Armada serves the folder as it is and changes nothing in it. Setup opens for it next.</p>
      )}
    </Dialog>
  );
}

function FolderFields({ path, pathMessage, sending, onPath, onChoose }: LocateFormProps) {
  return (
    <div className="armada-locate__row">
      <Input
        label="Project location"
        mono
        value={path}
        disabled={sending}
        invalid={pathMessage !== undefined}
        message={pathMessage}
        onChange={(event) => onPath(event.target.value)}
      />
      <Button variant="secondary" disabled={sending} onClick={() => onChoose("path")}>
        Choose a folder
      </Button>
    </div>
  );
}

function CloneFields({ url, parent, landsIn, parentMessage, sending, onUrl, onParent, onChoose }: LocateFormProps) {
  return (
    <>
      <Input label="Repository URL" mono value={url} disabled={sending} onChange={(event) => onUrl(event.target.value)} />
      <div className="armada-locate__row">
        <Input
          label="Clone into"
          mono
          value={parent}
          disabled={sending}
          invalid={parentMessage !== undefined}
          message={parentMessage}
          onChange={(event) => onParent(event.target.value)}
        />
        <Button variant="secondary" disabled={sending} onClick={() => onChoose("parent")}>
          Choose a folder
        </Button>
      </div>
      {/* Read-only, and the label the journey names: where the project will be. */}
      <div className="armada-locate__lands" role="group" aria-label="Project location">
        <span className="armada-locate__label">Project location</span>
        {landsIn === null ? (
          <span className="armada-locate__pending">A new folder, named from the URL</span>
        ) : (
          <span className="armada-locate__path">{landsIn}</span>
        )}
      </div>
    </>
  );
}
