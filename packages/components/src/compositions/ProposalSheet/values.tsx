// The two values on the sheet a cell cannot hold, each opened from where it sits. Its own file
// so the sheet stays one screen of layout.

import { Fragment, type ReactNode } from "react";

import { OneFlag, OrderedPicks, ValuePopover, type OrderedPick } from "../ValuePopover/ValuePopover";
import type { ProposalEntry, ProposalSheetProps } from "./ProposalSheet";

/** Written or already set up: a value is shown, not offered. Busy keeps a popover open and still. */
function frozen(props: ProposalSheetProps): boolean {
  return props.written !== undefined || props.setUp === true;
}

/**
 * Only Commands are offered: a gate that runs to set up a gate is no longer a gate. A set value reads
 * before the command it runs ahead of and the offer after it, so an unset row keeps no gap. Keyed,
 * so the first tick moves the command rather than remounting the popover it was ticked in.
 */
export function RunsFirst({ entry, children, ...props }: ProposalSheetProps & { entry: ProposalEntry; children: ReactNode }) {
  const requires = entry.requires ?? [];
  const written = requires.length === 0 ? "" : `${requires.join(", ")} →`;
  const command = <Fragment key="command">{children}</Fragment>;
  const around = (value: ReactNode) => (written === "" ? [command, value] : [value, command]);
  if (frozen(props)) return written === "" ? command : around(<span key="value" className="armada-proposal-sheet__mono">{written}</span>);
  const declared = props.commands.map((command) => command.name);
  const options: OrderedPick[] = [
    ...props.commands.map((command) =>
      command.destructive === true
        ? { name: command.name, unavailable: "Destructive, so no Check may run it first." }
        : { name: command.name },
    ),
    ...requires
      .filter((name) => !declared.includes(name))
      .map((name) => ({ name, unavailable: "Not a Command this file declares." })),
  ];
  if (options.length === 0) return command;
  return around(
    <ValuePopover
      key="value"
      label={`${entry.name} runs first`}
      {...(written === "" ? { offer: "Add runs first", offerName: `Add runs first: ${entry.name}` } : { value: written })}
    >
      <OrderedPicks
        label={`Commands ${entry.name} runs first`}
        says="Ticked Commands run before this Check, in the order ticked. To reorder, untick and tick again."
        options={options}
        picked={requires}
        busy={props.busy}
        onPicked={(next) => props.onRequires(entry.name, next)}
      />
    </ValuePopover>
  );
}

/** A judgement nothing in the repository makes, so the popover says whose it is. */
export function Destructive({ entry, ...props }: ProposalSheetProps & { entry: ProposalEntry }) {
  const on = entry.destructive === true;
  if (frozen(props)) return on ? <span className="armada-proposal-sheet__says">destructive</span> : null;
  const runsIt = [
    ...props.checks.filter((check) => check.requires?.includes(entry.name) === true).map((check) => check.name),
    ...(props.setup?.requires.includes(entry.name) === true ? ["setup"] : []),
  ];
  return (
    <ValuePopover
      label={`${entry.name} destructive`}
      {...(on ? { value: "destructive" } : { offer: "Mark destructive", offerName: `Mark destructive: ${entry.name}` })}
    >
      <OneFlag
        checked={on}
        description="A Drone asks you before it runs this."
        judgement="Nothing Scan reads can tell what a script destroys, so this is your judgement."
        {...(runsIt.length === 0
          ? {}
          : { unavailable: `${runsIt.join(" and ")} ${runsIt.length === 1 ? "runs" : "run"} it first, so it cannot be destructive.` })}
        busy={props.busy}
        onChange={(next) => props.onDestructive(entry.name, next)}
      >
        Destructive
      </OneFlag>
    </ValuePopover>
  );
}
