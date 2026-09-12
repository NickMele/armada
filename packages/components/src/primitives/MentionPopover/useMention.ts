import { useEffect, useId, useRef, useState } from "react";
import type { AriaAttributes, ChangeEvent, KeyboardEvent, SyntheticEvent } from "react";

/**
 * The `@` mention a person is in the middle of typing, or `null` where the
 * cursor is not inside one.
 */
type Mention = {
  /** Where the `@` sits in the field's text. */
  at: number;
  /** What has been typed since the `@`, up to the cursor. */
  query: string;
};

/**
 * Drives the `@` mention popup a textarea offers while a person is writing a
 * request or a brief — read the field for the mention under the cursor, ask
 * `search` for what it narrows to, and turn a chosen path into text inserted
 * at the `@`.
 *
 * **No ref onto the field.** `Textarea` is a plain function component and
 * React 19 still needs a prop typed for a ref to forward one; every native
 * event already hands back the element as `event.target`, so this reads the
 * cursor off the event that is firing rather than holding one.
 *
 * **A mention is found by scanning left from the cursor, not by a trigger
 * key.** `@` opens it and any character on the built-in word-boundary set —
 * whitespace — closes it, which is what lets a person keep typing the path
 * and lets punctuation like `.` and `/` stay part of the query.
 */
export function useMention(
  value: string,
  onChange: (next: string) => void,
  search: (query: string) => Promise<readonly string[]>,
): {
  open: boolean;
  query: string;
  results: readonly string[];
  active: number;
  /** The popup's own id, which the field points `aria-controls` at. */
  listId: string;
  /** One row's id, which the field points `aria-activedescendant` at. */
  optionId: (index: number) => string;
  /** What the field spreads onto itself. Empty while the popup is closed. */
  fieldAria: AriaAttributes & { role?: "combobox" };
  onHover: (index: number) => void;
  onFieldChange: (event: ChangeEvent<HTMLTextAreaElement>) => void;
  onFieldKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  onFieldSelect: (event: SyntheticEvent<HTMLTextAreaElement>) => void;
  onFieldBlur: () => void;
  onChoose: (path: string) => void;
} {
  const [mention, setMention] = useState<Mention | null>(null);
  const [results, setResults] = useState<readonly string[]>([]);
  const [active, setActive] = useState(0);
  // Answers a search that is no longer the one on screen are dropped rather
  // than drawn — a slow answer to "ru" must not overwrite the empty box a
  // fast "run" already narrowed to nothing.
  const asked = useRef(0);
  // Captured off the last field event, purely to put the cursor back after an
  // insertion — see `onChoose`. Not a ref onto `Textarea`, which this module's
  // own note says a plain function component cannot take in this React.
  const field = useRef<HTMLTextAreaElement | null>(null);
  // Two fields on one screen open this — the request composer and the dispatch
  // panel — so the ids the field and the popup agree on are per-instance
  // rather than constants, which is where `CommandPalette` can stop because
  // only one palette exists at a time.
  const baseId = useId();
  const listId = `${baseId}-mention-list`;
  const optionId = (index: number): string => `${baseId}-mention-option-${index}`;

  useEffect(() => {
    if (mention === null) {
      setResults([]);
      return;
    }
    const requestId = ++asked.current;
    let live = true;
    void search(mention.query).then((found) => {
      if (live && requestId === asked.current) setResults(found);
    });
    return () => {
      live = false;
    };
  }, [mention, search]);

  useEffect(() => setActive(0), [results]);

  function evaluate(el: HTMLTextAreaElement): void {
    const cursor = el.selectionStart ?? el.value.length;
    const upToCursor = el.value.slice(0, cursor);
    const at = upToCursor.lastIndexOf("@");
    if (at === -1 || /\s/.test(upToCursor.slice(at + 1))) {
      setMention(null);
      return;
    }
    // The `@` opens a mention only where it starts a word — preceded by
    // nothing or by whitespace — so an email typed into the same field is
    // never read as one.
    if (at > 0 && !/\s/.test(upToCursor[at - 1] ?? "")) {
      setMention(null);
      return;
    }
    setMention({ at, query: upToCursor.slice(at + 1) });
  }

  function onFieldChange(event: ChangeEvent<HTMLTextAreaElement>): void {
    field.current = event.target;
    onChange(event.target.value);
    evaluate(event.target);
  }

  function onFieldSelect(event: SyntheticEvent<HTMLTextAreaElement>): void {
    field.current = event.currentTarget;
    evaluate(event.currentTarget);
  }

  function onFieldKeyDown(event: KeyboardEvent<HTMLTextAreaElement>): void {
    field.current = event.currentTarget;
    if (mention === null) return;
    // **Escape first, and before anything counts the results.** It used to sit
    // last, behind a guard that returned on an empty list, so the one state a
    // person most wants dismissed — "No file matches" — was the one state the
    // key could not close.
    if (event.key === "Escape") {
      event.preventDefault();
      setMention(null);
      return;
    }
    if (results.length === 0) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((n) => (n + 1) % results.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((n) => (n - 1 + results.length) % results.length);
    } else if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      onChoose(results[active] ?? results[0] ?? "");
    }
  }

  /**
   * Leaving the field closes the popup. A row press never reaches this — the
   * popup prevents its own `mousedown`, so the field keeps focus through a
   * choice — which leaves this for the cases that should close it: a click
   * elsewhere, a tab out, a window that lost focus.
   */
  function onFieldBlur(): void {
    setMention(null);
  }

  function onChoose(path: string): void {
    if (mention === null || path === "") return;
    const cursor = mention.at + 1 + mention.query.length;
    const inserted = `@${path} `;
    const next = `${value.slice(0, mention.at)}${inserted}${value.slice(cursor)}`;
    onChange(next);
    setMention(null);
    // Next tick: the value above has not reached the DOM node yet on this
    // one, and setting the range now would place it against the text the
    // field is about to be replaced out from under.
    const el = field.current;
    if (el !== null) {
      const landed = mention.at + inserted.length;
      requestAnimationFrame(() => {
        el.focus();
        el.setSelectionRange(landed, landed);
      });
    }
  }

  // **Only while the popup is open.** A brief is prose first, and a field left
  // as a combobox for the whole time somebody writes one is announced as a
  // combobox rather than as the multi-line field it mostly is.
  // `CommandPalette`'s input carries the role permanently because searching is
  // the only thing that input does.
  const fieldAria: AriaAttributes & { role?: "combobox" } =
    mention === null
      ? {}
      : {
          role: "combobox",
          "aria-expanded": true,
          "aria-controls": listId,
          "aria-activedescendant": results.length > 0 ? optionId(active) : undefined,
          "aria-autocomplete": "list",
        };

  return {
    open: mention !== null,
    query: mention?.query ?? "",
    results,
    active,
    listId,
    optionId,
    fieldAria,
    onHover: setActive,
    onFieldChange,
    onFieldKeyDown,
    onFieldSelect,
    onFieldBlur,
    onChoose,
  };
}
