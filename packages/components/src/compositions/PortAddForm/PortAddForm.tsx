import { useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";

/**
 * Adding a port — Journey 3's *Adding a port*, one component with two mounts: Setup's
 * proposal sheet and the Manifest forms.
 *
 * **Two fields asked and a third filled in.** The variable starts from the name and stays
 * editable, so it can match what the code already reads. A variable another workspace maps
 * warns under the field, never elsewhere. **Data in, callbacks out.**
 */

export type PortAddition = { name: string; container: string; env: string };

export type PortAddFormProps = {
  /** Port names already declared here. A taken name is refused in its own field. */
  taken: readonly string[];
  /** A variable another workspace in the batch maps a port to, and that workspace. */
  elsewhere?: readonly { env: string; dir: string }[];
  onAdd: (port: PortAddition) => void;
  disabled?: boolean;
};

/** The starting value in both mounts — uppercase, `_PORT` suffix — or one port gets two names. */
export function portVariableOf(name: string): string {
  const stem = name
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  if (stem === "") return "";
  return stem === "PORT" || stem.endsWith("_PORT") ? stem : `${stem}_PORT`;
}

const PORT = /^[0-9]+$/;

export function PortAddForm({ taken, elsewhere = [], onAdd, disabled = false }: PortAddFormProps) {
  const [name, setName] = useState("");
  const [container, setContainer] = useState("");
  // `null` until a person types one: until then the variable follows the name.
  const [typedEnv, setTypedEnv] = useState<string | null>(null);
  const trimmed = name.trim();
  const env = typedEnv ?? portVariableOf(trimmed);
  const clash = taken.includes(trimmed);
  const number = Number(container);
  const badPort = container !== "" && (!PORT.test(container) || number < 1 || number > 65535);
  const shared = elsewhere.find((other) => other.env === env && env !== "");
  const ready = trimmed !== "" && !clash && container !== "" && !badPort && !disabled;

  return (
    <div className="armada-port-add" role="group" aria-label="Add a port">
      <div className="armada-port-add__fields">
        <Input
          label="Service name"
          mono
          value={name}
          invalid={clash}
          message={clash ? `A port named ${trimmed} is already declared here.` : undefined}
          onChange={(event) => setName(event.target.value)}
        />
        <Input
          label="Container port"
          mono
          inputMode="numeric"
          value={container}
          invalid={badPort}
          message={badPort ? "A port is a whole number from 1 to 65535." : undefined}
          onChange={(event) => setContainer(event.target.value)}
        />
        <div className="armada-port-add__variable">
          <Input
            label="Variable"
            mono
            value={env}
            onChange={(event) => setTypedEnv(event.target.value)}
          />
          {shared === undefined ? null : (
            <p className="armada-port-add__warning">
              {`${shared.dir} also maps a port to ${env}. That breaks only when one Job gates both.`}
            </p>
          )}
          <p className="armada-port-add__hint">
            Code reads it from the environment, or a command writes it as a variable for a tool that
            takes a port flag. Never write a literal host and port into a Check.
          </p>
        </div>
      </div>
      <Button
        variant="secondary"
        size="sm"
        disabled={!ready}
        onClick={() => {
          onAdd({ name: trimmed, container, env });
          setName("");
          setContainer("");
          setTypedEnv(null);
        }}
      >
        Add a port
      </Button>
    </div>
  );
}
