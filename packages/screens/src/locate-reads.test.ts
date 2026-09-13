// The folder a clone lands in, named as Fleet names it — `cloning.rs`'s own cases, and the parent.

import { describe, expect, it } from "vitest";

import { folderNamedBy, isAbsolute, landsIn } from "./locate-reads";

describe("the folder a clone makes", () => {
  it.each([
    ["https://forge.invalid/owner/storefront.git", "storefront"],
    ["https://forge.invalid/owner/storefront", "storefront"],
    ["https://forge.invalid/owner/storefront/", "storefront"],
    ["https://forge.invalid/owner/storefront/.git", "storefront"],
    ["git@forge.invalid:owner/storefront.git", "storefront"],
    ["git@host:storefront.git", "storefront"],
    ["file:///tmp/remotes/scratch.git", "scratch"],
    ["  /srv/git/api.git  ", "api"],
  ])("names %s %s", (url, name) => {
    expect(folderNamedBy(url)).toBe(name);
  });

  it.each(["", "   ", "host:", "/", ".git", "host:..", "a/."])("names no folder for %j", (url) => {
    expect(folderNamedBy(url)).toBeNull();
  });
});

describe("where a clone lands", () => {
  it("joins the parent and the folder, whatever the parent's trailing slash", () => {
    expect(landsIn("https://forge.invalid/owner/storefront.git", "/Users/user/code/")).toBe("/Users/user/code/storefront");
    expect(landsIn("git@host:api.git", "/Users/user/code")).toBe("/Users/user/code/api");
  });

  it("is nothing where the parent is relative or blank, or the URL names no folder", () => {
    expect(landsIn("https://forge.invalid/owner/storefront.git", "code")).toBeNull();
    expect(landsIn("https://forge.invalid/owner/storefront.git", "")).toBeNull();
    expect(landsIn("git@host:", "/Users/user/code")).toBeNull();
    expect(isAbsolute(" /Users/user")).toBe(true);
  });
});
