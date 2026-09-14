/**
 * This machine's own settings — Helm's action authority first (#1089).
 *
 * **Read-only, on purpose.** Each is resolved once, when Fleet starts, from
 * `crates/config/settings.toml`; nothing here sends an override while Fleet
 * runs, so a row is a label and a sentence rather than a field and a Save.
 */
export type MachineSettingsRow = {
  label: string;
  /** What the setting does, and when Fleet decides it. */
  means: string;
};

export type MachineSettingsProps = {
  rows: readonly MachineSettingsRow[];
};

export function MachineSettings({ rows }: MachineSettingsProps) {
  return (
    <div className="armada-machine-settings">
      {rows.map((row) => (
        <div className="armada-machine-settings__field" key={row.label}>
          <p className="armada-machine-settings__label">{row.label}</p>
          <p className="armada-machine-settings__means">{row.means}</p>
        </div>
      ))}
    </div>
  );
}
