import type { ReactNode } from "react";
import { useId, useState } from "react";
import { Chapter, type ChapterTone } from "../Chapter/Chapter";

/**
 * The step's story — Drone instructions, then Activity log, then Produced, in
 * the order it happened.
 *
 * **Opening one chapter collapses the others to their header line.** Not to
 * save room: to keep the story's order intact while one part of it is long. A
 * collapsed chapter still carries its header, so what happened in the step
 * stays readable at a glance even while a reader is deep in one part of it.
 *
 * **One open at a time**, and that is the constraint that makes this different
 * from a stack of accordions — six open at once was unreadable, which is what
 * this replaces. The cost is real and stated rather than hidden: you cannot
 * read the transcript beside the diff, and that pairing is how a Drone
 * narrating one thing and doing another is caught. If that turns out to matter
 * the fix is a split for those two only, not letting everything open at once.
 *
 * **Every chapter is always on screen.** A chapter with nothing open shows its
 * preview — the first few log entries, the files with their counts — so the
 * story reads through without pressing anything. Opening is for the whole of a
 * thing, never for finding out that it exists.
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
   * The header line's trailing half — `47 entries · every line opens`,
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

  return (
    <ol className="armada-story">
      {chapters.map((chapter) => {
        const opens = chapter.content !== undefined;
        const shown = open === chapter.id;
        // A chapter that is not the open one collapses to its header the
        // moment anything is open. Nothing else changes: same chapters, same
        // order, so the reader never loses where they are.
        const collapsed = open !== null && !shown;
        return (
          <li className="armada-story__chapter" key={chapter.id} data-open={shown || undefined}>
            <Chapter
              ordinal={chapter.ordinal}
              name={chapter.title}
              meta={chapter.summary}
              live={chapter.live}
              tone={chapter.tone}
              open={!collapsed}
              onToggle={opens ? () => toggle(chapter.id) : undefined}
              act={chapter.act}
              bodyId={`${bodies}-${chapter.id}`}
              moreLabel={
                !opens ? undefined : shown ? (chapter.closeLabel ?? "Close") : (chapter.openLabel ?? "Open")
              }
              onMore={opens ? () => toggle(chapter.id) : undefined}
              moreCloses={shown}
            >
              {shown ? chapter.content : chapter.preview}
            </Chapter>
          </li>
        );
      })}
    </ol>
  );
}
