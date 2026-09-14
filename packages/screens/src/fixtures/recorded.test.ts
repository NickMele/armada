// Every recording is replayed, drawn, and has a story.
//
// **The third is the one a person would miss.** `scripts/record-job.mjs`
// rewrites the list of recordings, but a story is an export a person writes,
// and Storybook cannot draw one from a list. A recording with no story is a
// state somebody captured that nobody can look at, so it fails here rather
// than sitting unseen.

import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { JobDetail } from "../JobDetail";
import { renderFor } from "../render";
import { propsFor } from "./props";
import { RECORDED_SLUGS, recorded } from "./recorded";

// Split across sibling files by #1044, so every one of them is read rather
// than the single file this test used to name.
const STORIES_DIR = fileURLToPath(new URL("../stories/screens/JobDetail/", import.meta.url));
const STORIES = readdirSync(STORIES_DIR)
  .filter((name) => name.endsWith(".stories.tsx"))
  .map((name) => readFileSync(`${STORIES_DIR}${name}`, "utf8"))
  .join("\n");

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
