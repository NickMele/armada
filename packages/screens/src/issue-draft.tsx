// The issue drafted from a For context finding, for a person to edit before the forge files it. #906.

import { useState } from "react";
import { Dialog, Input, Textarea } from "@armada/components";
import type { IssueDraft } from "./followup";

export type IssueDraftDialogProps = {
  draft: IssueDraft;
  onFile: (title: string, body: string) => void;
  onCancel: () => void;
};

/**
 * **Filed as the person, so nothing reaches the forge until they confirm.** Enter in the title
 * files, as in every dialog that collects; Enter in the body writes a new line, and ⌘Enter files.
 */
export function IssueDraftDialog({ draft, onFile, onCancel }: IssueDraftDialogProps) {
  const [title, setTitle] = useState(draft.title);
  const [body, setBody] = useState(draft.body);
  return (
    <Dialog
      open
      title="File an issue"
      tone="neutral"
      width="wide"
      confirmLabel="File the issue"
      confirmDisabled={title.trim() === ""}
      onConfirm={() => onFile(title.trim(), body)}
      onCancel={onCancel}
      field={
        <>
          <Input label="Title" value={title} onChange={(event) => setTitle(event.target.value)} />
          <Textarea
            label="Body"
            rows={8}
            value={body}
            onChange={(event) => setBody(event.target.value)}
            onKeyDown={(event) => {
              // Dialog's window handler files on Enter; a body needs its new lines.
              if (event.key === "Enter" && !event.metaKey) event.stopPropagation();
            }}
          />
        </>
      }
    >
      <p>Armada files this on the repository&apos;s forge as you. Nothing is filed until you confirm.</p>
    </Dialog>
  );
}
