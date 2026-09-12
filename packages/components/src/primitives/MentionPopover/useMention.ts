import { useEffect, useRef, useState } from "react";
import type { ChangeEvent, KeyboardEvent, SyntheticEvent } from "react";

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
  onHover: (index: number) => void;
  onFieldChange: (event: ChangeEvent<HTMLTextAreaElement>) => void;
  onFieldKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void;
  onFieldSelect: (event: SyntheticEvent<HTMLTextAreaElement>) => void;
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
    if (mention === null || results.length === 0) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((n) => (n + 1) % results.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((n) => (n - 1 + results.length) % results.length);
    } else if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      onChoose(results[active] ?? results[0] ?? "");
    } else if (event.key === "Escape") {
      event.preventDefault();
      setMention(null);
    }
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

  return {
    open: mention !== null,
    query: mention?.query ?? "",
    results,
    active,
    onHover: setActive,
    onFieldChange,
    onFieldKeyDown,
    onFieldSelect,
    onChoose,
  };
}
