import { useId, useState } from "react";
import type { KeyboardEvent } from "react";
import { ChevronDown } from "lucide-react";

import { Input } from "../../primitives/Input/Input";

/**
 * A branch field that offers the repository's branches and still takes a name
 * typed by hand.
 *
 * **Typed as well as picked, because the list is a floor.** Nothing on the
 * wire lists a repository's refs — `packages/screens/src/draft/branches.ts`
 * says what fills it and what it leaves out — so a picker that refused
 * everything it had not seen would refuse most of a real repository.
 *
 * **`offerNew` is the one difference between the two fields it draws.** Where
 * the work starts has to exist already; where it lands may not, and naming a
 * branch that is not there is how one gets made.
 */
export type BranchOption = {
  name: string;
  /** The Manifest's base: where a worktree is cut from, and what the field opens on. */
  base?: boolean;
  /** The Job holding it. Said beside the name, never instead of it. */
  job?: string;
};

export type BranchPickerProps = {
  /** Sentence case, no Wh- opener. Names the field and the list it opens. */
  label: string;
  value: string;
  onValue: (name: string) => void;
  /**
   * The repository's branches. **`null` is nothing having listed them**, which
   * draws the plain field this was before — an empty picker over a repository
   * nobody read says there are no branches, which is a different claim.
   */
  branches: readonly BranchOption[] | null;
  /** A name matching nothing is a branch to make. Off, it is just a name. */
  offerNew?: boolean;
  disabled?: boolean;
};

/** What a row says about a branch nothing has cut yet. */
const NEW = "new branch";

/** What the base row says. The word `armada.yml` uses, so the two agree. */
const BASE = "base";

export function BranchPicker({
  label,
  value,
  onValue,
  branches,
  offerNew = false,
  disabled = false,
}: BranchPickerProps) {
  const listId = useId();
  const optionId = (index: number): string => `${listId}-${index}`;
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);

  // Nothing listed them, so there is nothing to open. Bridge on a real Fleet
  // is here until a read exists, and the field it draws is the one it drew
  // before there was a picker at all.
  if (branches === null) {
    return (
      <Input
        label={label}
        mono
        value={value}
        disabled={disabled}
        onChange={(event) => onValue(event.target.value)}
      />
    );
  }

  // A value resting on a branch narrows to nothing, which would open the list
  // on the one row a person already has. Narrowing starts when they type past
  // it.
  const named = branches.some((one) => one.name === value);
  const query = named ? "" : value.trim().toLowerCase();
  const matches =
    query === "" ? branches : branches.filter((one) => one.name.toLowerCase().includes(query));
  const making = offerNew && value.trim() !== "" && !named;
  const rows = matches.length + (making ? 1 : 0);

  function close(): void {
    setOpen(false);
    setActive(0);
  }

  function choose(index: number): void {
    const picked = index < matches.length ? matches[index]?.name : value.trim();
    if (picked === undefined || picked === "") return;
    onValue(picked);
    close();
  }

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>): void {
    // Escape closes the list and nothing else. Left unprevented while the list
    // is shut, so the same key still leaves the composer the field sits in.
    if (event.key === "Escape") {
      if (!open) return;
      event.preventDefault();
      close();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (!open) {
        setOpen(true);
        setActive(0);
      } else if (rows > 0) {
        setActive((n) => (n + 1) % rows);
      }
      return;
    }
    if (event.key === "ArrowUp") {
      if (!open) return;
      event.preventDefault();
      if (rows > 0) setActive((n) => (n - 1 + rows) % rows);
      return;
    }
    if (event.key === "Enter" && open && rows > 0) {
      event.preventDefault();
      choose(active);
      return;
    }
    if (event.key === "Tab") close();
  }

  return (
    <div className="armada-branch">
      <Input
        label={label}
        mono
        value={value}
        disabled={disabled}
        role="combobox"
        aria-expanded={open}
        aria-controls={listId}
        aria-activedescendant={open && rows > 0 ? optionId(active) : undefined}
        aria-autocomplete="list"
        autoComplete="off"
        trailing={<ChevronDown className="armada-branch__caret" size={16} strokeWidth={2} aria-hidden />}
        onChange={(event) => {
          onValue(event.target.value);
          setOpen(true);
          setActive(0);
        }}
        onFocus={() => setOpen(true)}
        // A row press never reaches this: the row prevents its own
        // `mousedown`, so the field keeps focus through a choice.
        onBlur={close}
        onKeyDown={onKeyDown}
      />
      {!open ? null : (
        <div className="armada-branch__list" id={listId} role="listbox" aria-label={label}>
          {rows === 0 ? (
            <p className="armada-branch__empty">{`No branch Armada has met matches “${value}”.`}</p>
          ) : (
            <>
              {matches.map((branch, index) => (
                <Row
                  key={branch.name}
                  id={optionId(index)}
                  active={index === active}
                  name={branch.name}
                  {...(branch.base === true ? { tag: BASE } : {})}
                  {...(branch.base !== true && branch.job !== undefined ? { job: branch.job } : {})}
                  onHover={() => setActive(index)}
                  onChoose={() => choose(index)}
                />
              ))}
              {!making ? null : (
                <Row
                  id={optionId(matches.length)}
                  active={active === matches.length}
                  name={value.trim()}
                  tag={NEW}
                  onHover={() => setActive(matches.length)}
                  onChoose={() => choose(matches.length)}
                />
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * One branch, and what there is to say about it.
 *
 * **A tag and a Job's title are two kinds of thing, so they give way
 * differently.** `base` and `new branch` are what the row is telling you and
 * hold their width; a title is a nicety and truncates before the name does.
 * A row carries one of the two or neither.
 */
function Row({
  id,
  active,
  name,
  tag,
  job,
  onHover,
  onChoose,
}: {
  id: string;
  active: boolean;
  name: string;
  /** The base, or that nothing has cut this branch yet. */
  tag?: string;
  /** The Job whose worktree is on it. */
  job?: string;
  onHover: () => void;
  onChoose: () => void;
}) {
  const said = tag ?? job;
  return (
    <div
      id={id}
      role="option"
      aria-selected={active}
      className={active ? "armada-branch__row armada-branch__row--active" : "armada-branch__row"}
      // A branch name is longer than a field, so the row truncates and the
      // pointer is what reaches what truncation hid — the rule the Job Board's
      // own field run follows.
      title={said === undefined ? name : `${name} · ${said}`}
      onMouseEnter={onHover}
      // `onMouseDown`, not `onClick`: a click fires after the field's own blur,
      // which has already closed this list.
      onMouseDown={(event) => {
        event.preventDefault();
        onChoose();
      }}
    >
      <span className="armada-branch__name mono">{name}</span>
      {tag !== undefined ? <span className="armada-branch__tag">{tag}</span> : null}
      {tag === undefined && job !== undefined ? (
        <span className="armada-branch__on">{job}</span>
      ) : null}
    </div>
  );
}
