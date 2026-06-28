import { Trans } from "@lingui/solid/macro";
import { PageShell } from "./PageShell";

export function NotFoundPage() {
  return (
    <PageShell eyebrow="Missing" title="Page not found">
      <p>
        <Trans>The requested page does not exist.</Trans>
      </p>
    </PageShell>
  );
}
