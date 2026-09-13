import { Alert } from "../../primitives/Alert/Alert";
import { Input } from "../../primitives/Input/Input";

import { Section } from "./Entries";
import type { ManifestFormProps } from "./ManifestForm";

export function BudgetSection({ draft, onDraft, problems, budgetWarnings }: ManifestFormProps) {
  return (
    <Section title="Budget" says="What one Job here may spend. Empty takes what Fleet runs with.">
      <Input
        label="Cost cap per Job, in dollars"
        mono
        inputMode="decimal"
        value={draft.costCap}
        invalid={problems["budget.cost"] !== undefined}
        message={problems["budget.cost"]}
        onChange={(event) => onDraft({ ...draft, costCap: event.target.value })}
      />
      <Input
        label="Turn cap per Job"
        mono
        inputMode="numeric"
        value={draft.turnCap}
        invalid={problems["budget.turns"] !== undefined}
        message={problems["budget.turns"]}
        onChange={(event) => onDraft({ ...draft, turnCap: event.target.value })}
      />
      {budgetWarnings.map((warning) => (
        <Alert key={warning} tone="caution" title="Below what a past Job cost">
          {warning}
        </Alert>
      ))}
    </Section>
  );
}
