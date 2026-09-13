// Locate, held by the app — `docs/journeys/set-up-a-project-manifest.md`, *Getting in*. Above
// the dialog, so a clone still running when it closes is still running when it opens again.

import { useEffect, useRef, useState } from "react";
import { Alert, Button, LocateForm, type LocateFormProps, type LocateMode } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

import { said } from "./copy";
import { DESTINATION_OCCUPIED, isAbsolute, landsIn, type LocateAnswer } from "./locate-reads";

/** What Locate asks of the host. */
export type LocateSlice = {
  onChooseFolder: () => Promise<string | null>;
  /** A clone parent as Fleet will canonicalise it, resolved in main. `null` where it is no folder. */
  onResolveFolder: (path: string) => Promise<string | null>;
  onAdd: (path: string) => Promise<LocateAnswer>;
  onClone: (url: string, parent: string) => Promise<LocateAnswer>;
  /**
   * Served: pick it and go to Setup. **Called only on a person's press** — the send while the
   * dialog is up, or the notice's Open Setup where it finished after the dialog closed.
   */
  onLocated: (repository: RepositorySummary) => void;
  /** Fleet has listed nothing: the dialog opens by itself, since there is nothing else to do. */
  nothingServed?: boolean;
  /** A clone main heard land, from any window. Announced here unless this window's own send is taking it (#926). */
  landed?: { repository: RepositorySummary; at: number } | null;
};

export type Locating = {
  open: boolean;
  onOpen: () => void;
  form: LocateFormProps;
  /** A repository located after its dialog closed, until the person opens its Setup or dismisses it. */
  late: RepositorySummary | null;
  onLate: (open: boolean) => void;
};

const FULL_PATH = "A full path, starting with /.";

export function useLocate(slice: LocateSlice): Locating {
  const [open, setOpen] = useState(false);
  const [mode, setMode] = useState<LocateMode>("folder");
  const [path, setPath] = useState("");
  const [url, setUrl] = useState("");
  const [parent, setParent] = useState("");
  const [sending, setSending] = useState(false);
  const [refusal, setRefusal] = useState<LocateFormProps["refusal"]>(undefined);
  const [late, setLate] = useState<RepositorySummary | null>(null);
  const [resolved, setResolved] = useState<{ parent: string; folder: string | null } | null>(null);
  const latest = useRef(slice);
  latest.current = slice;
  const shown = useRef(open);
  shown.current = open;
  const mountedAt = useRef(Date.now());
  // This window's own send, and what it located: that answer is this window's to act on, not to announce.
  const own = useRef<{ out: boolean; took: string | null }>({ out: false, took: null });

  // Heard after this window opened, and not its own: said as late, which moves no pick.
  useEffect(() => {
    const heard = slice.landed;
    if (heard == null || heard.at < mountedAt.current) return;
    if (own.current.out || heard.repository.root === own.current.took) return;
    setLate(heard.repository);
  }, [slice.landed]);

  useEffect(() => {
    if (slice.nothingServed === true) setOpen(true);
  }, [slice.nothingServed]);

  // Fleet canonicalises the parent, so the preview asks main for the same spelling.
  useEffect(() => {
    const asked = parent.trim();
    if (!isAbsolute(asked)) return undefined;
    let current = true;
    void latest.current.onResolveFolder(asked).then((folder) => {
      if (current) setResolved({ parent: asked, folder });
    });
    return () => {
      current = false;
    };
  }, [parent]);
  const canonical = resolved?.parent === parent.trim() ? resolved.folder : null;
  const destination = landsIn(url, canonical ?? parent);
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
    own.current.out = true;
    setRefusal(undefined);
    const cloning = mode === "clone";
    try {
      const answer = cloning ? await latest.current.onClone(url.trim(), parent.trim()) : await latest.current.onAdd(path.trim());
      if (answer.state === "located") {
        own.current.took = answer.repository.root;
        const wasOpen = shown.current;
        reset();
        setOpen(false);
        if (wasOpen) latest.current.onLocated(answer.repository);
        else setLate(answer.repository);
        return;
      }
      setRefusal(refusalOf(answer, cloning));
    } finally {
      own.current.out = false;
      setSending(false);
    }
  }

  return {
    open,
    onOpen: () => setOpen(true),
    late,
    onLate: (going) => {
      if (going && late !== null) latest.current.onLocated(late);
      setLate(null);
    },
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
function refusalOf(
  answer: Exclude<LocateAnswer, { state: "located" }>,
  cloning: boolean,
): NonNullable<LocateFormProps["refusal"]> {
  switch (answer.state) {
    case "refused":
      return {
        title: titleOf(answer.code, cloning),
        saying: sentence(answer.saying),
        ...(answer.code === DESTINATION_OCCUPIED ? { next: "Choose another folder to clone into." } : {}),
      };
    case "busy":
      return { title: "Not sent", saying: "A repository is already being added or cloned." };
    case "failed":
      return { title: answer.outcome.ok === false && answer.outcome.why === "transport" ? "No answer from Fleet" : "Not sent", saying: said(answer.outcome) };
  }
}

/** Fleet's own clone codes mean nothing landed; any other refusal of a clone came after git, and the clone stays. */
const CLONE_CODES = new Set(["fleet.clone_refused", DESTINATION_OCCUPIED, "fleet.no_such_folder", "fleet.clone_not_finished"]);

function titleOf(code: string, cloning: boolean): string {
  if (!cloning) return "Not added";
  return CLONE_CODES.has(code) ? "Not cloned" : "Cloned, and not added";
}

/** Fleet's messages end bare; the dialog reads them as sentences. */
function sentence(saying: string): string {
  const trimmed = saying.trim();
  return /[.!?]$/.test(trimmed) ? trimmed : `${trimmed}.`;
}

/** The dialog, drawn from what `useLocate` holds. */
export function Locate({ locating }: { locating: Locating }) {
  return <LocateForm {...locating.form} />;
}

/**
 * A clone that finished after its dialog closed, said wherever the person is. **It never moves the
 * window**: Open Setup is theirs to press, and until then the pick stays where they left it.
 */
export function LocatedNotice({ locating, repositories }: { locating: Locating; repositories: readonly RepositorySummary[] }) {
  const { late } = locating;
  if (late === null) return null;
  const listed = repositories.some((one) => one.root === late.root) ? repositories : [...repositories, late];
  return (
    <Alert
      tone="neutral"
      title={`${repositoryLabel(late, listed)} is ready to set up`}
      action={
        <>
          <Button variant="secondary" size="sm" onClick={() => locating.onLate(true)}>
            Open Setup
          </Button>
          <Button variant="ghost" size="sm" onClick={() => locating.onLate(false)}>
            Dismiss
          </Button>
        </>
      }
    >
      {`The clone into ${late.root} finished, and Armada serves it. It is in the project picker.`}
    </Alert>
  );
}
