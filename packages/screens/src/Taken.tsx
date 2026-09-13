// A press a freeze took, drawn. Neutral, because the press worked: nothing was refused.

import { Alert, Button } from "@armada/components";

export function TakenNotice({ title, body, onDismiss }: { title: string; body: string; onDismiss?: () => void }) {
  return (
    <Alert
      tone="neutral"
      title={title}
      action={
        onDismiss === undefined ? undefined : (
          <Button variant="ghost" size="sm" onClick={onDismiss}>
            Dismiss
          </Button>
        )
      }
    >
      {body}
    </Alert>
  );
}
