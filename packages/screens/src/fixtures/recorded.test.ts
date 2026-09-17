// Every recording is replayed and drawn. The mock opens each one as
// `recorded/<slug>` and a row of `every-state` with no edit, so none needs a
// story to be looked at — #1224.

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { JobDetail } from "../JobDetail";
import { renderFor } from "../render";
import { propsFor } from "./props";
import { RECORDED_SLUGS, recorded } from "./recorded";

describe.each(RECORDED_SLUGS)("the recording %s", (slug) => {
  it("replays into a Job this build can render", () => {
    expect(renderFor(recorded(slug).job)).not.toBe("unrenderable");
  });

  it("draws as the whole screen without throwing", () => {
    const fixture = recorded(slug);
    expect(() => renderToStaticMarkup(createElement(JobDetail, propsFor(fixture)))).not.toThrow();
  });
});
