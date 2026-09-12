/**
 * A plain-language reading of one command a drone reached for, for the person
 * deciding whether to allow it. Since protocol 11.5. Mirrors
 * `ipc::CommandExplained` in `crates/ipc/src/explaining.rs`.
 *
 * **It decides nothing.** The offers a command carries are unchanged, and a
 * person who never asks is answered exactly as before.
 */
export type CommandExplained = {
  /**
   * The reading itself, in prose a person can act on: what the command does,
   * and what about it is worth a second look.
   */
  explanation: string;
  /**
   * Which model said it. Named because the reading is a claim, not a fact —
   * a person weighing it is entitled to know what read it.
   */
  model: string;
};
