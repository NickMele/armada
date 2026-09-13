// The pieces every section of the Manifest form is built from.
import { useId, useState, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";

export function Section({ title, says, children }: { title: string; says: string; children: ReactNode }) {
  return (
    <section className="armada-manifest-form__section" aria-label={title}>
      <h2 className="armada-manifest-form__title">{title}</h2>
      <p className="armada-manifest-form__says">{says}</p>
      {children}
    </section>
  );
}

/**
 * One named entry, grouped under its name, with Remove beside it. A removal
 * takes the entry's lines and the comment directly above it — the writer's.
 */
export function Entry({ name, onRemove, children }: { name: string; onRemove: () => void; children: ReactNode }) {
  const id = useId();
  return (
    <fieldset className="armada-manifest-form__entry" aria-labelledby={id}>
      <div className="armada-manifest-form__entry-head">
        <span id={id} className="armada-manifest-form__name">
          {name}
        </span>
        <Button variant="ghost" size="sm" onClick={onRemove}>
          Remove
        </Button>
      </div>
      {children}
    </fieldset>
  );
}

/** A name, and the press that declares an entry by it. Taken names are refused here. */
export function AddEntry({ noun, taken, onAdd }: { noun: string; taken: string[]; onAdd: (name: string) => void }) {
  const [typed, setTyped] = useState("");
  const name = typed.trim();
  const clash = taken.includes(name);
  return (
    <div className="armada-manifest-form__add">
      <Input
        label={`New ${noun} name`}
        mono
        value={typed}
        invalid={clash}
        message={clash ? `This file already declares ${name}.` : undefined}
        onChange={(event) => setTyped(event.target.value)}
      />
      <Button
        variant="secondary"
        size="sm"
        disabled={name === "" || clash}
        onClick={() => {
          onAdd(name);
          setTyped("");
        }}
      >
        {`Add a ${noun}`}
      </Button>
    </div>
  );
}

export function replaced<T>(list: readonly T[], at: number, next: T): T[] {
  return list.map((one, index) => (index === at ? next : one));
}

/** A field and the line that says how to fill it, shown whether or not it is valid. */
export function Hinted({ hint, children }: { hint: string; children: ReactNode }) {
  return (
    <div className="armada-manifest-form__hinted">
      {children}
      <p className="armada-manifest-form__hint">{hint}</p>
    </div>
  );
}
