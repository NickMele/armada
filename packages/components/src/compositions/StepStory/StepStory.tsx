import type { ReactNode } from "react";
import { useId, useState } from "react";
import { Chapter, type ChapterTone } from "../Chapter/Chapter";

/**
 * The step's story — Drone instructions, then Activity log, then Produced, in
 * the order it happened.
 *
 * **A header opens and closes its own chapter and no other.** A collapsed
 * chapter keeps its header, so the story's order and what happened in the step
 * stay readable at a glance, and a reader decides how much else is on screen.
 * Showing a chapter's whole content, where it has more than its preview, is its
 * foot control's and the keyboard map's, and that stays one chapter at a time.
 *
 * **Every chapter starts open, on its preview**: the first few log entries, the
 * files with their counts. The story reads through without pressing anything.
 *
 * **A chapter is `Chapter`, and this holds the order and the one-open rule.**
 * It drew its own header for a while, beside a `Chapter` component that was
 * drawing the same header better — two answers to one question, and the visible
 * cost was the activity log's live indicator: the word "live" in a summary
 * string, where the drawing and `Chapter` both have the running dot. A claim
 * that something is still arriving is not a word in a count.
 */
export type StepChapter = {
  id: string;
  /**
   * Its place in the story, drawn in the mark. The order is the order things
   * happened and it never changes, which is what a reader navigates by.
   */
  ordinal: number;
  title: ReactNode;
  /**
   * The header line's trailing half — `47 entries`,
   * `3 files · +94 −31`. What the chapter holds, so a collapsed one still
   * answers for itself.
   */
  summary?: ReactNode;
  /**
   * Whether the chapter is streaming. Draws the running dot before the summary,
   * which is what says the activity log is live rather than a snapshot — the
   * one claim a count cannot make, and the one a caller must not spell as the
   * word `live` inside `summary`.
   */
  live?: boolean;
  /**
   * What the chapter is. `waiting` is the one that asks rather than reports —
   * the decision chapter on a step stopped at a human gate.
   */
  tone?: ChapterTone;
  /**
   * What the header says on hover, where what pressing it does is the
   * interesting half — `Click to view the files`. Absent looks the chapter's
   * name up in the vocabulary instead; see `ChapterProps.says`.
   */
  says?: ReactNode;
  /** What it shows while nothing in the story is open. */
  preview?: ReactNode;
  /**
   * What it shows while it is the open one. Absent on a chapter that has no
   * more to give than its preview — which makes it un-openable, and the
   * control does not draw.
   */
  content?: ReactNode;
  /**
   * The control on the chapter's header line — `Open the log`, `Open the diff`.
   *
   * **A chapter whose content has no end opens as a trailing sheet instead**,
   * because the panel cannot hold it and the chapter line is the way back. Such
   * a chapter carries no `content`: it has a preview, and an act that leaves.
   */
  act?: ReactNode;
  /** The control that opens it — `Open the log — all 47 entries`. */
  openLabel?: ReactNode;
  /** The control that closes it. `Close` unless a chapter wants its own word. */
  closeLabel?: ReactNode;
};

export type StepStoryProps = {
  chapters: StepChapter[];
  /** Which chapter is open on mount. After that the story holds its own. */
  openId?: string;
  /**
   * Which chapter is open, held by the caller. **Present makes the story
   * controlled**: it draws what this says and changes nothing itself, and
   * `onOpen` is the only way the value moves. `null` is a story with every
   * chapter collapsed to its preview.
   *
   * This exists so a keyboard map can open a chapter by name instead of
   * reaching into the DOM for the component's own class names. Absent leaves
   * the story uncontrolled, which is what every caller that only clicks wants.
   */
  openChapter?: string | null;
  /** Told when a chapter is opened or closed, for a caller that records it. */
  onOpen?: (chapterId: string | null) => void;
};

export function StepStory({ chapters, openId, openChapter, onOpen }: StepStoryProps) {
  const [held, setHeld] = useState<string | null>(openId ?? null);
  // Which chapters a reader folded by their header. Local: folding is how much
  // of the story is on screen, and nothing above this needs to know it.
  const [shut, setShut] = useState<ReadonlySet<string>>(() => new Set());
  const bodies = useId();
  // Controlled by presence, not by a flag: a caller either holds the value or
  // it does not, and a boolean beside it is a second answer that can disagree.
  const controlled = openChapter !== undefined;
  const open = controlled ? openChapter : held;

  function toggle(chapterId: string): void {
    const next = open === chapterId ? null : chapterId;
    if (!controlled) setHeld(next);
    onOpen?.(next);
  }

  function fold(chapterId: string): void {
    setShut((was) => {
      const next = new Set(was);
      if (next.has(chapterId)) next.delete(chapterId);
      else next.add(chapterId);
      return next;
    });
  }

  return (
    <ol className="armada-story">
      {chapters.map((chapter) => {
        // `more` is whether there is anything past the preview, which is what
        // draws the control at the foot. `opens` is whether the header can be
        // pressed, which any chapter with something to show can be.
        const more = chapter.content !== undefined;
        const opens = more || chapter.preview !== undefined;
        const shown = open === chapter.id;
        // Folded by its own header and by nothing else. A chapter showing its
        // whole content is open whatever its header last said.
        const collapsed = shut.has(chapter.id) && !shown;
        return (
          <li className="armada-story__chapter" key={chapter.id} data-open={shown || undefined}>
            <Chapter
              ordinal={chapter.ordinal}
              name={chapter.title}
              meta={chapter.summary}
              live={chapter.live}
              tone={chapter.tone}
              says={chapter.says}
              open={!collapsed}
              onToggle={opens ? () => fold(chapter.id) : undefined}
              act={chapter.act}
              bodyId={`${bodies}-${chapter.id}`}
              moreLabel={
                !more ? undefined : shown ? (chapter.closeLabel ?? "Close") : (chapter.openLabel ?? "Open")
              }
              onMore={more ? () => toggle(chapter.id) : undefined}
              moreCloses={shown}
            >
              {shown ? (chapter.content ?? chapter.preview) : chapter.preview}
            </Chapter>
          </li>
        );
      })}
    </ol>
  );
}
