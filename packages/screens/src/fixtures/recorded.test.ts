// Every recording is replayed, drawn, and has a story.
//
// **The third is the one a person would miss.** `scripts/record-job.mjs`
// rewrites the list of recordings, but a story is an export a person writes,
// and Storybook cannot draw one from a list. A recording with no story is a
// state somebody captured that nobody can look at, so it fails here rather
// than sitting unseen.

import { readFileSync } from "node:fs";

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { JobDetail } from "../JobDetail";
import { renderFor } from "../render";
import { propsFor } from "./props";
import { RECORDED_SLUGS, recorded } from "./recorded";

const STORIES = readFileSync(
  new URL("../stories/screens/JobDetail/JobDetail.stories.tsx", import.meta.url),
  "utf8",
);

describe.each(RECORDED_SLUGS)("the recording %s", (slug) => {
  it("replays into a Job this build can render", () => {
    expect(renderFor(recorded(slug).job)).not.toBe("unrenderable");
  });

  it("draws as the whole screen without throwing", () => {
    const fixture = recorded(slug);
    expect(() => renderToStaticMarkup(createElement(JobDetail, propsFor(fixture)))).not.toThrow();
  });

  it("has a story", () => {
    expect(STORIES).toContain(`recorded("${slug}")`);
  });
});
