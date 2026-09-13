// A person's Bridge preferences, kept the way `limits.ts`'s three are kept —
// Fleet-wide, surviving a relaunch. `crates/ipc/src/preferences.rs`.
//
// The header rules in `protocol.ts` hold here: these are hand-written, and
// they drift the day a field moves.

/** Every preference Fleet knows, and what is in force for each — `GET
 * /preferences`. */
export type Preferences = {
  /** Whether Job detail's *Where things are* chapter opens collapsed
   * (`false`) or expanded (`true`). */
  where_things_are_open: boolean;
};

/**
 * A save — `POST /preferences/save`. **One preference, not the set** — unlike
 * `SaveLimits`, which sends every field it has an opinion on in one request.
 * `name` outside the closed set is refused by name, `fleet.unknown_preference`.
 */
export type SavePreference = {
  name: string;
  value: boolean;
};
