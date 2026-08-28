// Which editor a path is opened with, and where that answer comes from.
//
// `shell.openPath` asks LaunchServices, which for a **file** lands in whatever
// is bound to `.jsonl` and for a **directory** lands in Finder. A worktree is a
// directory, so the row #162 added opens Finder — a working answer, and not the
// one a person wanting to read the Drone's work asked for.
//
// # The precedence, and why it is three tiers
//
// | order | source | the question it answers |
// |---|---|---|
// | 1 | a config row | what a person decided, and will change from Bridge |
// | 2 | `$VISUAL`, then `$EDITOR` | what a developer already told every other tool |
// | 3 | `shell.openPath` | what happens when nobody has said anything |
//
// **Tier 1 is not here, and its absence is the finding rather than an
// omission.** `crates/config/settings.toml` is where such a row would go, and
// seven rows in it already carry `read_by = "bridge (TS)"` against no delivery
// mechanism at all — `docs/contracts/configuration.md` says so in as many
// words, and `[bridge-notification-routing-path]` in `docs/OPEN.md` is the
// same problem filed as a question. `crates/config` today reads five keys of an
// `armada.yml` and has no Machine layer, so a row added for this would be an
// eighth thing nothing resolves, which is exactly the defect
// `docs/practices/half-built.md` names. Tiers 2 and 3 stand on their own; when
// config can reach Bridge, `chooseEditor` gains one branch above the two here.
//
// # Nothing here reaches a shell
//
// The sanctioned way to hand a path to the OS is `shell.openPath`, and it is
// still what tier 3 does. There is no Electron call for *"open this with that
// application"*, so honouring a named editor means starting a process — and the
// whole of the care is in how.
//
// **`spawn` with an argv array, never a shell.** `shell` is left at its default
// `false`, the program is argv[0] and the path is the last argument, and
// nothing is ever concatenated into a string a shell would parse. A value with
// `;` or a backtick in it is a program name containing those characters, which
// will not be found, rather than a second command. That is the difference
// between this and the injection surface, and it is a property of the call
// rather than of a validation step that could be forgotten.
//
// **Quoting is not interpreted, on purpose.** `$EDITOR` is split on
// whitespace — `code -w` is the program `code` and the flag `-w` — and quotes
// are left as ordinary characters. Interpreting them is what a shell does.
//
// # Where these variables are, and are not
//
// A macOS app launched from the Dock inherits launchd's environment, which does
// not source a login shell: `$VISUAL` and `$EDITOR` set in `.zshrc` are not
// there. Launched from a terminal — which is how Bridge runs today — they are.
// So tier 2 is reachable rather than universal, and a Dock launch falls to tier
// 3 silently, which is the correct behaviour for a variable that is unset.

import { spawn } from "node:child_process";
import { access, stat } from "node:fs/promises";
import { constants } from "node:fs";

/**
 * An editor a person named, and which variable named it.
 *
 * `from` is carried because the sentence a failure produces has to say where
 * the value came from — *"`$EDITOR` names `code`"* sends a person to the right
 * line of the right file, and *"the editor is missing"* sends them nowhere.
 */
export type Editor = {
  /** The program, as it was written. */
  readonly command: string;
  /** Arguments written before the path, in order. */
  readonly args: readonly string[];
  /** Which variable it was read from. */
  readonly from: "$VISUAL" | "$EDITOR";
};

/** Whitespace-only is unset. An exported empty string is not a decision. */
function said(value: string | undefined): string[] | null {
  const words = (value ?? "").trim().split(/\s+/).filter((word) => word !== "");
  return words.length === 0 ? null : words;
}

/**
 * The editor to use, or `null` for *"nobody has said"*.
 *
 * `$VISUAL` before `$EDITOR` is the older convention and the right way round
 * here for the reason it exists: `$EDITOR` may name a line editor for a dumb
 * terminal, `$VISUAL` names the full-screen one, and Bridge is asking for the
 * second kind. A person with both set has already drawn that distinction.
 */
export function chooseEditor(env: NodeJS.ProcessEnv): Editor | null {
  for (const from of ["$VISUAL", "$EDITOR"] as const) {
    const words = said(env[from.slice(1)]);
    if (words === null) continue;
    const [command, ...args] = words;
    if (command === undefined) continue;
    return { command, args, from };
  }
  return null;
}

/**
 * Where that program actually is, or `null`.
 *
 * **Resolved before anything is started, for the same reason `openArtifact`
 * stats the path first**: a spawn that fails with `ENOENT` reports itself as
 * `spawn code ENOENT`, which is a fact about Node rather than a sentence for a
 * person, and it arrives on an event after the call has already returned.
 *
 * A command carrying a `/` is a path and is taken as one — absolute only,
 * because the main process's working directory is wherever Electron was
 * started and a relative path resolved against it means nothing a person
 * chose. Otherwise `PATH` is walked in order, which is the lookup the shell
 * that set the variable would have done.
 */
export async function whereIs(command: string, env: NodeJS.ProcessEnv): Promise<string | null> {
  const executable = async (candidate: string): Promise<boolean> => {
    try {
      if (!(await stat(candidate)).isFile()) return false;
      await access(candidate, constants.X_OK);
      return true;
    } catch {
      return false;
    }
  };

  if (command.includes("/")) {
    if (!command.startsWith("/")) return null;
    return (await executable(command)) ? command : null;
  }

  for (const directory of (env.PATH ?? "").split(":")) {
    if (directory === "") continue;
    const candidate = `${directory}/${command}`;
    if (await executable(candidate)) return candidate;
  }
  return null;
}

/**
 * Start the editor on that path, detached, and stop caring about it.
 *
 * **Detached and unreferenced, with no stdio**, because an editor outlives the
 * click that opened it: a child of Bridge would be a window that closes when
 * Bridge closes, and a pipe nobody reads is a child that blocks on a full
 * buffer. Bridge's job ends at *started*.
 *
 * Which means **failures after the start are not Bridge's to report** — an
 * editor that launches and then complains does so in its own window. What is
 * reported is the one failure Bridge can see and a person can act on, which
 * `whereIs` has already answered before this is called.
 */
export function startEditor(program: string, args: readonly string[], path: string): void {
  const child = spawn(program, [...args, path], {
    detached: true,
    stdio: "ignore",
    // Named rather than defaulted. `shell: true` here would hand the whole
    // argv to `/bin/sh` and turn a configured string into a command line.
    shell: false,
  });
  // A spawn can still fail asynchronously — a binary that is executable and is
  // not one, say. Nothing listens, and an unhandled `error` on a ChildProcess
  // takes the main process down, so the handler exists to be the one that does
  // nothing.
  child.on("error", () => {});
  child.unref();
}
