// The arithmetic behind the screens, tested without drawing anything.
//
// **Two runners, and the split is deliberate.** `packages/components` runs its
// stories in a browser, because what a component owes is a rendering. The
// modules here owe an answer: which tab a Job is in, which render it takes,
// what a terminal Job does to the step beneath it. Those are functions of the
// wire, so they are tested as functions — no DOM to start, no story to write,
// and a hundred cases cost what one costs.
//
// **`node`, not `jsdom`.** A test here that needed a document would be a test
// of a component in the wrong package.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    name: "screens",
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
