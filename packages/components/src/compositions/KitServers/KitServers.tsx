import { useId, useState, type FormEvent, type ReactNode } from "react";
import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../../primitives/Table/Table";

/**
 * The MCP servers in a person's kit, and what a drone dispatched against this
 * repository is handed. #1275.
 *
 * **Two tiers, drawn as two controls.** Kit holds the default across every
 * repository; this repository's manifest extends or restricts it, and saying
 * nothing is a third state rather than an absent second — a person who has
 * withheld a server here has to be able to get back to following kit.
 *
 * **Adding a server turns nothing on.** A new row arrives reaching no drone
 * and says so, which is the confinement `docs/scope.md` keeps: a drone comes
 * up holding what it was given and nothing else.
 */
export type KitServersProps = {
  /**
   * Every server in kit, by name. `undefined` is not yet read, and draws
   * "Reading." rather than an empty table — which would say a person's kit is
   * empty before anybody asked.
   */
  servers?: KitServerRowProps[];
  /** Put one in kit. The form clears once the list comes back with it. */
  onAdd?: (adding: { name: string; kind: KitServerKind; address: string }) => void;
  /** Take one out, and this repository's word about it with it. */
  onForget?: (name: string) => void;
  /** Kit's own tier, across every repository that has not said otherwise. */
  onKitReach?: (name: string, reaches: boolean) => void;
  /** This repository's word, or `null` to follow kit again. */
  onHereReach?: (name: string, reach: "extended" | "restricted" | null) => void;
  /** Fleet's own refusal, read exactly. Cleared by whatever it answers next. */
  refused?: ReactNode;
};

/** A program fleet starts, or an address a drone opens. */
export type KitServerKind = "stdio" | "http";

/** One server, with both tiers and the answer they come to. */
export type KitServerRowProps = {
  name: string;
  /** The program and its arguments, or the URL — one line, as it was given. */
  address: string;
  kind: KitServerKind;
  /** Kit's default, across every repository. */
  reachesByDefault: boolean;
  /** This repository's word. Absent where it has none, and kit's default answers. */
  here?: "extended" | "restricted";
  /** Whether a drone dispatched here gets it. Fleet's own answer, never worked out here. */
  resolves: boolean;
};

const KIND_LABEL: Record<KitServerKind, string> = {
  stdio: "Program",
  http: "Address",
};

const HERE_LABEL: Record<"kit" | "extended" | "restricted", string> = {
  kit: "Follow Kit",
  extended: "Allowed here",
  restricted: "Withheld here",
};

export function KitServers({
  servers,
  onAdd,
  onForget,
  onKitReach,
  onHereReach,
  refused,
}: KitServersProps) {
  const group = useId();
  const [name, setName] = useState("");
  const [kind, setKind] = useState<KitServerKind>("stdio");
  const [address, setAddress] = useState("");

  function add(event: FormEvent): void {
    event.preventDefault();
    if (name.trim() === "" || address.trim() === "") return;
    onAdd?.({ name: name.trim(), kind, address: address.trim() });
    setName("");
    setAddress("");
  }

  return (
    <div className="armada-kit-servers">
      <p className="armada-kit-servers__lead">
        The MCP servers you have connected. A server added here reaches no drone until you allow
        it — in Kit for every repository, or in this one alone. A drone already running keeps what
        it started with.
      </p>

      {refused === undefined ? null : (
        <Alert tone="caution" title="That was refused">
          {refused}
        </Alert>
      )}

      <form className="armada-kit-servers__add" onSubmit={add}>
        <Input
          id={`${group}-name`}
          label="Name"
          value={name}
          placeholder="github"
          onChange={(event) => setName(event.target.value)}
        />
        <Select
          id={`${group}-kind`}
          label="Kind"
          value={kind}
          onChange={(event) => setKind(event.target.value as KitServerKind)}
        >
          <option value="stdio">Program</option>
          <option value="http">Address</option>
        </Select>
        <Input
          id={`${group}-address`}
          label={KIND_LABEL[kind]}
          value={address}
          placeholder={kind === "stdio" ? "npx -y @scope/server" : "https://example.com/mcp"}
          onChange={(event) => setAddress(event.target.value)}
        />
        <Button type="submit" variant="secondary" size="sm">
          Add
        </Button>
      </form>

      {servers === undefined ? (
        <p className="armada-kit-servers__reading">Reading your Kit.</p>
      ) : servers.length === 0 ? (
        <p className="armada-kit-servers__reading">
          Nothing in your Kit yet. A drone here gets Armada&rsquo;s own tool and nothing else.
        </p>
      ) : (
        <Table className="armada-kit-servers__table">
          <TableHead>
            <TableRow>
              <TableHeaderCell>Server</TableHeaderCell>
              <TableHeaderCell>Where it is</TableHeaderCell>
              <TableHeaderCell>In Kit</TableHeaderCell>
              <TableHeaderCell>Here</TableHeaderCell>
              <TableHeaderCell>A drone here</TableHeaderCell>
              <TableHeaderCell>{""}</TableHeaderCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {servers.map((server) => (
              <TableRow key={server.name}>
                <TableCell variant="primary">{server.name}</TableCell>
                <TableCell variant="mono">{server.address}</TableCell>
                <TableCell variant="secondary">
                  <Select
                    aria-label={`In Kit: ${server.name}`}
                    value={server.reachesByDefault ? "yes" : "no"}
                    onChange={(event) => onKitReach?.(server.name, event.target.value === "yes")}
                  >
                    <option value="no">Off</option>
                    <option value="yes">On everywhere</option>
                  </Select>
                </TableCell>
                <TableCell variant="secondary">
                  <Select
                    aria-label={`Here: ${server.name}`}
                    value={server.here ?? "kit"}
                    onChange={(event) =>
                      onHereReach?.(
                        server.name,
                        event.target.value === "kit"
                          ? null
                          : (event.target.value as "extended" | "restricted"),
                      )
                    }
                  >
                    <option value="kit">{HERE_LABEL.kit}</option>
                    <option value="extended">{HERE_LABEL.extended}</option>
                    <option value="restricted">{HERE_LABEL.restricted}</option>
                  </Select>
                </TableCell>
                {/* Fleet's own resolution, said plainly. Not a badge: the
                    badge roster is the Job state machine's. */}
                <TableCell variant="secondary">
                  <span
                    className="armada-kit-servers__resolves"
                    data-resolves={server.resolves || undefined}
                  >
                    {server.resolves ? "Gets it" : "Does not"}
                  </span>
                </TableCell>
                <TableCell variant="secondary">
                  {onForget === undefined ? null : (
                    <Button
                      variant="secondary"
                      size="sm"
                      ground="sunken"
                      aria-label={`Remove ${server.name}`}
                      onClick={() => onForget(server.name)}
                    >
                      Remove
                    </Button>
                  )}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </div>
  );
}
