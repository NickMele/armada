// Shared across this directory's story files, so a split by group never
// duplicates what one of them would otherwise define twice. #1044.
import type { StoryObj } from "@storybook/react-vite";

import type { JobFixture } from "../../../fixtures/fixture";
import { JobDetailFrom } from "./JobDetail";

/** A story that draws one fixture, with no controls. A whole Job is not an arg. */
export function drawing(fixture: () => JobFixture): StoryObj<typeof JobDetailFrom>["render"] {
  return () => <JobDetailFrom fixture={fixture()} />;
}
