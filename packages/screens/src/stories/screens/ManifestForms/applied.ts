// A form's edits, applied to what a story's fake Fleet declares — as far as a
// play test presses them. Fleet's writer is what places an edit for real.

import type { EvidenceDraft, ManifestDeclared, ManifestEdit } from "@armada/protocol";

/** `declared` with `key` set, or taken out where the value is null or an empty list. */
function keyed<K extends keyof ManifestDeclared>(
  declared: ManifestDeclared,
  key: K,
  value: ManifestDeclared[K] | null,
): ManifestDeclared {
  const { [key]: _dropped, ...rest } = declared;
  const gone = value === null || value === undefined || (Array.isArray(value) && value.length === 0);
  return (gone ? rest : { ...rest, [key]: value }) as ManifestDeclared;
}

function evidenceWith(declared: ManifestDeclared, change: Partial<Record<keyof EvidenceDraft, unknown>>): ManifestDeclared {
  if (declared.evidence === undefined) return declared;
  const merged = Object.entries({ ...declared.evidence, ...change }).filter(
    ([, value]) => value !== null && !(Array.isArray(value) && value.length === 0),
  );
  return { ...declared, evidence: Object.fromEntries(merged) as EvidenceDraft };
}

export function appliedTo(declared: ManifestDeclared, edits: readonly ManifestEdit[]): ManifestDeclared {
  let next = declared;
  const checked = (name: string, change: (check: ManifestDeclared["checks"][number]["check"]) => object) => ({
    ...next,
    checks: next.checks.map((one) => (one.name === name ? { ...one, check: { ...one.check, ...change(one.check) } } : one)),
  });
  for (const edit of edits) {
    switch (edit.edit) {
      case "add_check":
        next = { ...next, checks: [...next.checks, { name: edit.name, check: edit.check }] };
        break;
      case "remove_check":
        next = { ...next, checks: next.checks.filter((one) => one.name !== edit.name) };
        break;
      case "set_check_run":
        next = checked(edit.name, () => ({ run: edit.run }));
        break;
      case "set_check_expect_exit_code":
        next = checked(edit.name, () => ({ expect_exit_code: edit.expect_exit_code }));
        break;
      case "add_command":
        next = { ...next, commands: [...next.commands, { name: edit.name, command: edit.command }] };
        break;
      case "remove_command":
        next = { ...next, commands: next.commands.filter((one) => one.name !== edit.name) };
        break;
      case "set_cost_cap_micros_per_job":
        next = keyed(next, "cost_cap_micros_per_job", edit.cost_cap_micros_per_job);
        break;
      case "set_review_gate":
        next = { ...next, review_gate: { ...next.review_gate, written: edit.review_gate ?? "human_always" } };
        break;
      case "set_freeze":
        next = { ...next, freeze: edit.freeze };
        break;
      case "set_base":
        next = keyed(next, "base", edit.base);
        break;
      case "add_evidence":
        next = keyed(next, "evidence", edit.evidence);
        break;
      case "remove_evidence":
        next = keyed(next, "evidence", null);
        break;
      case "set_evidence_serve":
        next = evidenceWith(next, { serve: edit.serve });
        break;
      case "set_evidence_ready":
        next = evidenceWith(next, { ready: edit.ready });
        break;
      case "set_evidence_run":
        next = evidenceWith(next, { run: edit.run });
        break;
      case "set_evidence_frames":
        next = evidenceWith(next, { frames: edit.frames });
        break;
      case "set_evidence_never":
        next = evidenceWith(next, { never: edit.never });
        break;
      case "set_after_merge_checks":
        next = keyed(next, "after_merge_checks", edit.checks);
        break;
      case "set_setup_requires":
        next = keyed(next, "setup_requires", edit.requires);
        break;
      case "set_quiet_after_seconds":
        next = keyed(next, "quiet_after_seconds", edit.quiet_after_seconds);
        break;
      case "set_poke_limit":
        next = keyed(next, "poke_limit", edit.poke_limit);
        break;
      case "set_exclude_paths":
        next = keyed(next, "exclude_paths", edit.exclude_paths);
        break;
      default:
        break;
    }
  }
  return next;
}
