// Helm's conversation zone, under the dock's questions — #944. `App.tsx` only
// wires this in; which repository Helm answers for is main's own decision,
// made in `main/helm.ts`, and this draws whatever it publishes.

import { useState } from "react";
import { HelmComposer, HelmThread } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { helmRowsOf } from "@armada/screens/src/helm-thread";
import type { BridgeState } from "../../shared/bridge";

export type HelmDockProps = {
  helm: BridgeState["helm"];
  repositories: readonly RepositorySummary[];
  /** The rail's own pick. `null` is All repositories — the only scope the switch draws in. */
  scope: string | null;
  live: boolean;
  onAsk: (text: string) => void;
  onStartFresh: () => void;
  onSwitch: (manifestId: string) => void;
};

export function HelmDock({ helm, repositories, scope, live, onAsk, onStartFresh, onSwitch }: HelmDockProps) {
  const [draft, setDraft] = useState("");
  const current = helm.state === "none" ? undefined : helm.manifestId;
  const options = repositories
    .filter((one): one is RepositorySummary & { manifest: NonNullable<RepositorySummary["manifest"]> } =>
      one.manifest !== undefined,
    )
    .map((one) => ({ id: one.manifest.id, label: one.manifest.repository }));
  const rows = helm.state === "open" || helm.state === "failed" ? helmRowsOf(helm.items) : [];
  const replying = helm.state === "open" && helm.replying;

  const notice = !live
    ? "Fleet is not connected. What Helm already said is still here."
    : helm.state === "failed"
      ? helm.detail
      : undefined;
  const emptyNote =
    helm.state === "cleared"
      ? "The conversation is cleared. Ask Helm something to start again."
      : current === undefined
        ? "No repository has a Manifest yet for Helm to answer about."
        : undefined;

  function send(): void {
    const text = draft.trim();
    if (text === "") return;
    onAsk(text);
    setDraft("");
  }

  return (
    <div className="armada-helm-dock">
      <HelmThread rows={rows} replying={replying} notice={notice} emptyNote={emptyNote} />
      <HelmComposer
        current={current}
        repositories={options}
        onSwitch={scope === null && options.length > 1 ? onSwitch : undefined}
        onStartFresh={onStartFresh}
        startFreshDisabled={replying}
        value={draft}
        onChange={setDraft}
        onSend={send}
        disabled={!live || current === undefined}
      />
    </div>
  );
}
