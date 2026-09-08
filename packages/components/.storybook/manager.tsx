// A reviewer's note against whatever story they are looking at, filed beside
// Controls and A11y rather than in a separate app — review and reply live in
// one loop, and a note is half of that loop's write side.
import React, { useCallback, useEffect, useState } from "react";
import { addons, types, useChannel, useStorybookApi, useStorybookState } from "storybook/manager-api";
import { styled } from "storybook/theming";
import { Button } from "storybook/internal/components";
import { PICK_CANCEL, PICK_CANCELLED, PICK_RESULT, PICK_START } from "./picker-types.ts";
import type { Picked } from "./picker-types.ts";

const ADDON_ID = "armada/notes";
const PANEL_ID = `${ADDON_ID}/panel`;

interface NoteRecord {
  story: string;
  titled: string;
  note: string;
  at: string;
  picked?: Picked;
}

// The panel is a fixed-height region handed to us by Storybook — content
// past that height must scroll inside it, not push the composer off the
// bottom. Column flex with one scrolling child is what makes that true.
const Wrap = styled.div(({ theme }) => ({
  display: "flex",
  flexDirection: "column",
  height: "100%",
  padding: 12,
  fontFamily: theme.typography.fonts.base,
  color: theme.color.defaultText,
}));

const Composer = styled.div({
  display: "flex",
  flexDirection: "column",
  gap: 8,
  flexShrink: 0,
});

const List = styled.div({
  flex: 1,
  minHeight: 0,
  overflowY: "auto",
  marginTop: 8,
});

const Textarea = styled.textarea(({ theme }) => ({
  minHeight: 80,
  resize: "vertical",
  fontFamily: "inherit",
  fontSize: theme.typography.size.s2,
  background: theme.background.content,
  color: theme.color.defaultText,
  border: `1px solid ${theme.appBorderColor}`,
  borderRadius: theme.appBorderRadius,
  padding: 8,
}));

const NoteRow = styled.div(({ theme }) => ({
  borderBottom: `1px solid ${theme.appBorderColor}`,
  padding: "6px 0",
  fontSize: theme.typography.size.s2,
}));

const Timestamp = styled.div(({ theme }) => ({
  fontSize: theme.typography.size.s1,
  color: theme.color.mediumdark,
}));

const PickRow = styled.div({
  display: "flex",
  gap: 8,
  alignItems: "center",
});

// A send that did not land. Loud on purpose: the failure it reports used to be
// silent, and a reviewer wrote several notes into a void before noticing.
const Failed = styled.div(({ theme }) => ({
  border: `1px solid ${theme.color.negative}`,
  borderRadius: theme.appBorderRadius,
  padding: 8,
  fontSize: theme.typography.size.s1,
  color: theme.color.negativeText ?? theme.color.negative,
}));

// What a pick holds, shown before Send so a reviewer can confirm they
// marked the right thing rather than trusting a rectangle they dragged fast.
const PickCard = styled.div(({ theme }) => ({
  border: `1px solid ${theme.appBorderColor}`,
  borderRadius: theme.appBorderRadius,
  padding: 8,
  fontSize: theme.typography.size.s1,
  display: "flex",
  flexDirection: "column",
  gap: 4,
}));

const ClassCode = styled.code(({ theme }) => ({
  fontSize: theme.typography.size.s1,
  background: theme.background.app,
  padding: "1px 4px",
  borderRadius: 3,
}));

function PickSummary({ picked }: { picked: Picked }) {
  return (
    <PickCard>
      <div>
        {picked.mode} · {picked.elements.length} element{picked.elements.length === 1 ? "" : "s"}
        {picked.elided ? ` · ${picked.elided} elided` : ""}
      </div>
      {picked.elements.map((el, index) => (
        <div key={index}>
          <ClassCode>
            {el.tag}
            {el.classes.length ? `.${el.classes.join(".")}` : ""}
          </ClassCode>
          {el.text && <span> — {el.text}</span>}
        </div>
      ))}
    </PickCard>
  );
}

function NotesPanel({ active }: { active?: boolean }) {
  const api = useStorybookApi();
  const { storyId } = useStorybookState();
  const [draft, setDraft] = useState("");
  const [notes, setNotes] = useState<NoteRecord[]>([]);
  const [picking, setPicking] = useState(false);
  const [picked, setPicked] = useState<Picked | null>(null);
  // Why the last send did not land. Null while nothing has failed.
  const [failed, setFailed] = useState<string | null>(null);

  const emit = useChannel({
    [PICK_RESULT]: (result: Picked) => {
      setPicked(result);
      setPicking(false);
    },
    [PICK_CANCELLED]: () => setPicking(false),
  });

  const refresh = useCallback(() => {
    if (!storyId) {
      setNotes([]);
      return;
    }
    fetch(`/__notes?story=${encodeURIComponent(storyId)}`)
      .then((res) => res.json())
      .then(setNotes)
      .catch(() => setNotes([]));
  }, [storyId]);

  useEffect(refresh, [refresh]);

  // A pick belongs to the story it was made against — carrying it into a
  // note on a different story would mislabel it.
  useEffect(() => {
    setPicked(null);
    setPicking(false);
  }, [storyId]);

  const startPicking = useCallback(() => {
    setPicked(null);
    setPicking(true);
    emit(PICK_START);
  }, [emit]);

  const cancelPicking = useCallback(() => {
    emit(PICK_CANCEL);
    setPicking(false);
  }, [emit]);

  // Right after the reviewer clicks "Pick an area", focus is still on that
  // button in the manager frame, not the preview iframe — the picker's own
  // Escape listener (`picker.ts`) only sees a key that reaches the iframe.
  // This is the other half, for a key pressed before the mouse ever moves.
  // Capture phase, ahead of Storybook's own shortcut handling, which stops
  // propagation on Escape for its own search/menu-closing behaviour.
  useEffect(() => {
    if (!picking) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") cancelPicking();
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, [picking, cancelPicking]);

  /**
   * Send the note, and **keep it in the box unless the server says it landed.**
   *
   * This cleared the draft on any settled promise and had no `catch`. A POST
   * that 404s while the dev server is restarting — which happens every time
   * anyone edits this directory, and happened repeatedly while a reviewer was
   * working — resolves with `ok: false`, so the note was wiped from the textarea
   * and never written. Silent, and indistinguishable from success: the reviewer
   * had written several before noticing none of them had arrived.
   *
   * A note is somebody's thinking. Losing one is worse than any amount of
   * chrome saying it failed.
   */
  const send = useCallback(() => {
    if (!storyId || !draft.trim()) return;
    const data = api.getCurrentStoryData();
    const titled = "title" in data ? `${data.title} / ${data.name}` : storyId;
    setFailed(null);
    fetch("/__notes", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ story: storyId, titled, note: draft, ...(picked ? { picked } : {}) }),
    })
      .then((response) => {
        if (!response.ok) throw new Error(`the dev server answered ${response.status}`);
        setDraft("");
        setPicked(null);
        setFailed(null);
        refresh();
      })
      .catch((reason: unknown) => {
        setFailed(reason instanceof Error ? reason.message : String(reason));
      });
  }, [api, draft, picked, refresh, storyId]);

  if (!active) return null;

  return (
    <Wrap>
      <Composer>
        <Textarea
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Note this story for the agent session reading .notes/storybook.jsonl"
        />
        <PickRow>
          {picking ? (
            <>
              <Button disabled>Hover to outline, click or drag, Esc to cancel…</Button>
              <Button onClick={cancelPicking}>Cancel</Button>
            </>
          ) : (
            <Button onClick={startPicking} disabled={!storyId}>
              Pick an area
            </Button>
          )}
        </PickRow>
        {picked && (
          <>
            <PickSummary picked={picked} />
            <Button onClick={() => setPicked(null)}>Clear pick</Button>
          </>
        )}
        <Button onClick={send} disabled={!draft.trim() || !storyId}>
          Send
        </Button>
        {/* Said in the panel rather than the console. The reviewer is looking
            at the textarea their note is still sitting in, which is the only
            place the failure means anything. */}
        {failed !== null && (
          <Failed role="alert">
            {`Not saved — ${failed}. Your note is still here; press Send again.`}
          </Failed>
        )}
      </Composer>
      <List>
        {notes.length === 0 && <Timestamp>No notes on this story yet.</Timestamp>}
        {notes.map((record, index) => (
          <NoteRow key={index}>
            <div>{record.note}</div>
            {record.picked && <PickSummary picked={record.picked} />}
            <Timestamp>{new Date(record.at).toLocaleString()}</Timestamp>
          </NoteRow>
        ))}
      </List>
    </Wrap>
  );
}

addons.register(ADDON_ID, () => {
  addons.add(PANEL_ID, {
    type: types.PANEL,
    title: "Notes",
    match: ({ viewMode }) => viewMode === "story",
    render: ({ active }) => <NotesPanel active={active} />,
  });
});
