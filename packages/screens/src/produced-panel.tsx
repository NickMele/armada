// The Produced panel, drawn from the current run's Produced chapter: its
// summary, its body and `Open the diff`. #1187.
import { ProducedPanel, type StepChapter } from "@armada/components";

import { DIFF_CHAPTER } from "./detail-keys";
import type { AttemptRead } from "./timeline";

/** Nothing where the step has no current run to build the chapter from. */
export function ProducedOf({ chapter }: { chapter: StepChapter | undefined }) {
  if (chapter === undefined) return null;
  return (
    <ProducedPanel
      {...(chapter.summary === undefined ? {} : { summary: chapter.summary })}
      {...(chapter.act === undefined ? {} : { act: chapter.act })}
    >
      {chapter.preview}
      {chapter.content}
    </ProducedPanel>
  );
}

/** A story builder that keeps the current run's Produced chapter aside, for the panel. */
export function keepingProduced(storyOf: (read: AttemptRead, over: boolean) => StepChapter[]) {
  let kept: StepChapter | undefined;
  return {
    storyOf: (read: AttemptRead, over: boolean): StepChapter[] => {
      const story = storyOf(read, over);
      if (!over) kept = story.find((chapter) => chapter.id === DIFF_CHAPTER);
      return story;
    },
    chapter: () => kept,
  };
}
