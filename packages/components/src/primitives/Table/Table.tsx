import type { HTMLAttributes, MouseEvent, TdHTMLAttributes, ThHTMLAttributes } from "react";
import { useCallback } from "react";

/**
 * Table — the densest thing in the app and the reason the spacing scale is
 * tight. This is the primitive: row chrome, the cell kinds and the three row
 * states. It is not the Job Board row, which is a stacked composition of a
 * badge, a headline sentence and a labelled field run.
 *
 * **Column tracks are not the primitive's.** The sheet settles this: the track
 * list belongs to the field set and not to the row, because two rows with
 * different fields cannot share one. A composition sets widths in its own
 * stylesheet; the primitive owns heights, rules, padding and cell kinds.
 *
 * No zebra striping. At 36px rows it reads as noise, and the row rule already
 * separates.
 */
export type TableProps = HTMLAttributes<HTMLTableElement>;

function joined(base: string, extra?: string) {
  return extra ? `${base} ${extra}` : base;
}

export function Table({ className, ...rest }: TableProps) {
  return <table className={joined("armada-table", className)} {...rest} />;
}

export function TableHead({ className, ...rest }: HTMLAttributes<HTMLTableSectionElement>) {
  return <thead className={joined("armada-table-head", className)} {...rest} />;
}

export function TableBody({ className, ...rest }: HTMLAttributes<HTMLTableSectionElement>) {
  return <tbody className={joined("armada-table-body", className)} {...rest} />;
}

export type TableRowProps = HTMLAttributes<HTMLTableRowElement> & {
  /** Selected: an `--accent-muted` fill. Coexists with focus. */
  selected?: boolean;
  /**
   * Focused: a 2px `--accent` left edge over `--bg-hover`. A 1px ring around
   * a full-width row is nearly invisible, so the row does something stronger,
   * and it is visible whenever the keyboard is driving rather than only on
   * `:focus-visible` — if a person is moving with j/k, the bar is the cursor.
   */
  focused?: boolean;
  /** De-emphasised: `--border-subtle` and `--fg-subtle`, never an alpha. */
  dimmed?: boolean;
};

export function TableRow({ className, selected, focused, dimmed, ...rest }: TableRowProps) {
  return (
    <tr
      className={joined("armada-table-row", className)}
      data-selected={selected || undefined}
      data-focused={focused || undefined}
      data-dimmed={dimmed || undefined}
      {...rest}
    />
  );
}

/**
 * The one legal ALL CAPS: `--text-2xs` at `0.04em` tracking, and only here.
 */
export function TableHeaderCell({ className, ...rest }: ThHTMLAttributes<HTMLTableCellElement>) {
  return <th scope="col" className={joined("armada-table-header-cell", className)} {...rest} />;
}

/**
 * `primary` is the thing the row is about. `secondary` is context. `metadata`
 * is a timestamp or an elapsed figure. `mono` is machine-derived and nothing
 * else — never prose.
 */
export type TableCellVariant = "primary" | "secondary" | "metadata" | "mono";

export type TableCellProps = TdHTMLAttributes<HTMLTableCellElement> & {
  variant?: TableCellVariant;
  /**
   * The exact string a mono cell puts on the clipboard. Setting it makes the
   * cell copy on click and go to `--accent` on hover. It carries no `copy`
   * glyph: the affordance token is the affordance, and a value that copies
   * does not also get a button that copies it.
   *
   * A clipboard write is silent, so a toast has to confirm it. The toast is
   * the surface's, not the primitive's — `onCopied` is where it hangs.
   *
   * The cell takes no tab stop. Its keyboard path is the row menu's Copy
   * entry, which carries its own binding; a tab stop on every mono value would
   * put four of them in each of fourteen rows.
   */
  copyValue?: string;
  onCopied?: (value: string) => void;
  /**
   * Truncate with an ellipsis rather than wrapping. The full string belongs in
   * a tooltip, which is the surface's. Needs a declared track width to
   * truncate against, and that width is the composition's.
   */
  truncates?: boolean;
};

export function TableCell({
  className,
  variant = "primary",
  copyValue,
  onCopied,
  truncates,
  onClick,
  ...rest
}: TableCellProps) {
  const copies = copyValue !== undefined;

  const handleClick = useCallback(
    (event: MouseEvent<HTMLTableCellElement>) => {
      onClick?.(event);
      if (copyValue === undefined) return;
      void navigator.clipboard.writeText(copyValue).then(
        () => onCopied?.(copyValue),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way and says which happened.
        () => onCopied?.(copyValue),
      );
    },
    [copyValue, onCopied, onClick],
  );

  return (
    <td
      className={joined("armada-table-cell", className)}
      data-variant={variant}
      data-copies={copies || undefined}
      data-truncates={truncates || undefined}
      onClick={handleClick}
      {...rest}
    />
  );
}
