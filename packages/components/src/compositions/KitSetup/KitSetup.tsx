import type { ReactNode } from "react";

/**
 * The setup a person already works with, read from their agent harness's own
 * home and shown by kind — #1491.
 *
 * **Seeing is the whole of it.** Nothing here is a control, because nothing
 * here reaches a drone: a server a person connected outside Armada is drawn
 * and still handed to nobody, and allowing one stays a separate act on a kit
 * row below. The rows carry no address for the same reason — a name, the
 * thing's own words for itself, and where it came from.
 *
 * **A kind nothing reads yet says so.** Drawing it as empty would say a person
 * has none of something they have plenty of, which is the report this whole
 * surface exists to stop giving.
 */
export type KitSetupProps = {
  /**
   * What the read came to. `undefined` is not yet read and draws "Reading." —
   * an empty setup drawn before the read lands is the wrong answer given by
   * accident, `KitServers`' reason.
   */
  setup?: KitSetupRead;
};

export type KitSetupRead = {
  /** The harness, in its own name. **Drawn, never matched on.** */
  harness: string;
  /** Where it was read, as a person would type it. */
  home: string;
  /** Whether that home is there at all. */
  present: boolean;
  /** Every kind, in the order fleet answered them. */
  kinds: KitSetupKind[];
};

/** Armada's word for one kind of thing, never a harness's. */
export type KitSetupKindWord =
  | "skills"
  | "plugins"
  | "agent_file"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "models";

export type KitSetupKind = {
  kind: KitSetupKindWord;
  read:
    | { what: "read"; items: KitSetupItem[]; unreadable: KitSetupUnreadable[] }
    | { what: "not_read"; why: string };
};

export type KitSetupItem = {
  name: string;
  /** Its own words for itself, where its file carries them. */
  says?: string;
  /** Where it came from. Drawn small, and never a control. */
  source: string;
};

export type KitSetupUnreadable = {
  source: string;
  why: string;
};

const KIND_LABEL: Record<KitSetupKindWord, string> = {
  skills: "Skills",
  plugins: "Plugins",
  agent_file: "Agent file",
  sub_agents: "Sub agents",
  commands: "Commands",
  mcp_servers: "MCP servers",
  allowlist: "Allowlist",
  models: "Models",
};

/**
 * What a kind is, where a person would not already know. The servers line is
 * the one that matters: it is the difference between seeing a server and a
 * drone getting one.
 */
const KIND_NOTE: Partial<Record<KitSetupKindWord, string>> = {
  mcp_servers: "Connected outside Armada. A drone here is handed none of them — allow one in Kit below and it is a Kit server from then on.",
  agent_file: "The global file saying how you want an agent to behave.",
};

/** A kind with nothing in it, in that kind's own words. */
const KIND_NONE: Record<KitSetupKindWord, string> = {
  skills: "No skills",
  plugins: "No plugins",
  agent_file: "No agent file",
  sub_agents: "No sub agents",
  commands: "No commands",
  mcp_servers: "No servers connected",
  allowlist: "Nothing always-allowed",
  models: "No models named",
};

export function KitSetup({ setup }: KitSetupProps) {
  if (setup === undefined) {
    return <p className="armada-kit-setup__reading">Reading what you already have.</p>;
  }
  return (
    <section className="armada-kit-setup" aria-label="What you already have">
      <header className="armada-kit-setup__head">
        <h3 className="armada-kit-setup__title">What you already have</h3>
        <p className="armada-kit-setup__where">
          {setup.harness}
          <span className="armada-kit-setup__home">{setup.home}</span>
        </p>
      </header>

      {setup.present ? null : (
        <p className="armada-kit-setup__reading">
          Nothing is there yet. Armada reads this each time you open Kit, so whatever you set up
          next shows up here.
        </p>
      )}

      {setup.kinds.map((kind) => (
        <Kind key={kind.kind} {...kind} />
      ))}
    </section>
  );
}

function Kind({ kind, read }: KitSetupKind) {
  const counted = read.what === "read" ? read.items.length : undefined;
  return (
    <div className="armada-kit-setup__kind">
      <h4 className="armada-kit-setup__kind-name">
        {KIND_LABEL[kind]}
        {counted === undefined ? null : (
          <span className="armada-kit-setup__count">{counted}</span>
        )}
      </h4>
      <Note>{KIND_NOTE[kind]}</Note>
      {read.what === "not_read" ? (
        /* Named as not read, never drawn as empty. */
        <p className="armada-kit-setup__not-read">Not read yet — {read.why}</p>
      ) : (
        <Items kind={kind} items={read.items} unreadable={read.unreadable} />
      )}
    </div>
  );
}

function Note({ children }: { children?: ReactNode }) {
  return children === undefined ? null : <p className="armada-kit-setup__note">{children}</p>;
}

function Items({
  kind,
  items,
  unreadable,
}: {
  kind: KitSetupKindWord;
  items: KitSetupItem[];
  unreadable: KitSetupUnreadable[];
}) {
  return (
    <>
      {items.length === 0 && unreadable.length === 0 ? (
        <p className="armada-kit-setup__none">{KIND_NONE[kind]}</p>
      ) : (
        <ul className="armada-kit-setup__items">
          {items.map((item) => (
            <li className="armada-kit-setup__item" key={`${item.source}/${item.name}`}>
              <span className="armada-kit-setup__item-name">{item.name}</span>
              {item.says === undefined ? null : (
                <span className="armada-kit-setup__item-says">{item.says}</span>
              )}
              <span className="armada-kit-setup__item-source">{item.source}</span>
            </li>
          ))}
        </ul>
      )}
      {unreadable.length === 0 ? null : (
        /* A file Armada could not describe is still one the person has, so it
           is named with the reason and nothing is offered to fix it here. */
        <ul className="armada-kit-setup__unreadable">
          {unreadable.map((one) => (
            <li key={one.source}>
              <span className="armada-kit-setup__item-name">{one.source}</span>
              <span className="armada-kit-setup__item-says">would not read: {one.why}</span>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
