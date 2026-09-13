// Locate, held by the app — `docs/journeys/set-up-a-project-manifest.md`, *Getting in*. Above
// the dialog, so a clone still running when it closes is still running when it opens again.

import { useRef, useState } from "react";
import { LocateForm, type LocateFormProps, type LocateMode } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { said } from "./copy";
import { DESTINATION_OCCUPIED, isAbsolute, landsIn, type LocateAnswer } from "./locate-reads";

/** What Locate asks of the host. */
export type LocateSlice = {
  onChooseFolder: () => Promise<string | null>;
  onAdd: (path: string) => Promise<LocateAnswer>;
  onClone: (url: string, parent: string) => Promise<LocateAnswer>;
  /** Served and picked: go to Setup. **Not called where the dialog was closed while it ran.** */
  onLocated: (repository: RepositorySummary) => void;
};

export type Locating = { open: boolean; onOpen: () => void; form: LocateFormProps };

const FULL_PATH = "A full path, starting with /.";

export function useLocate(slice: LocateSlice): Locating {
  const [open, setOpen] = useState(false);
  const [mode, setMode] = useState<LocateMode>("folder");
  const [path, setPath] = useState("");
  const [url, setUrl] = useState("");
  const [parent, setParent] = useState("");
  const [sending, setSending] = useState(false);
  const [refusal, setRefusal] = useState<LocateFormProps["refusal"]>(undefined);
  const latest = useRef(slice);
  latest.current = slice;
  const shown = useRef(open);
  shown.current = open;

  const destination = landsIn(url, parent);
  const ready = mode === "folder" ? isAbsolute(path) : url.trim() !== "" && destination !== null;
  const edited = <T,>(set: (value: T) => void) => (value: T) => {
    set(value);
    setRefusal(undefined);
  };

  function reset(): void {
    setMode("folder");
    setPath("");
    setUrl("");
    setParent("");
    setRefusal(undefined);
  }

  async function send(): Promise<void> {
    if (!ready || sending) return;
    setSending(true);
    setRefusal(undefined);
    const cloning = mode === "clone";
    try {
      const answer = cloning ? await latest.current.onClone(url.trim(), parent.trim()) : await latest.current.onAdd(path.trim());
      if (answer.state === "located") {
        const wasOpen = shown.current;
        reset();
        setOpen(false);
        if (wasOpen) latest.current.onLocated(answer.repository);
        return;
      }
      setRefusal(refusalOf(answer));
    } finally {
      setSending(false);
    }
  }

  return {
    open,
    onOpen: () => setOpen(true),
    form: {
      open,
      mode,
      path,
      url,
      parent,
      landsIn: destination,
      sending,
      ready,
      pathMessage: path.trim() !== "" && !isAbsolute(path) ? FULL_PATH : undefined,
      parentMessage: parent.trim() !== "" && !isAbsolute(parent) ? FULL_PATH : undefined,
      refusal,
      onMode: edited(setMode),
      onPath: edited(setPath),
      onUrl: edited(setUrl),
      onParent: edited(setParent),
      onChoose: (field) =>
        void latest.current.onChooseFolder().then((chosen) => {
          if (chosen === null) return;
          (field === "path" ? setPath : setParent)(chosen);
          setRefusal(undefined);
        }),
      onSend: () => void send(),
      // Closing sends nothing. A clone already out carries on in Fleet and still joins the picker.
      onCancel: () => {
        setOpen(false);
        if (!sending) reset();
      },
    },
  };
}

/** What an answer that did not locate says, in the dialog. */
function refusalOf(answer: Exclude<LocateAnswer, { state: "located" }>): NonNullable<LocateFormProps["refusal"]> {
  switch (answer.state) {
    case "refused":
      return {
        code: answer.code,
        saying: answer.saying,
        ...(answer.code === DESTINATION_OCCUPIED ? { next: "Choose another folder to clone into." } : {}),
      };
    case "busy":
      return { saying: "A repository is already being added or cloned. This one was not sent." };
    case "failed":
      return { saying: said(answer.outcome) };
  }
}

/** The dialog, drawn from what `useLocate` holds. */
export function Locate({ locating }: { locating: Locating }) {
  return <LocateForm {...locating.form} />;
}
