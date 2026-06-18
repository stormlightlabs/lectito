import { PageShell } from "./PageShell";

export function NotFoundPage() {
  return (
    <PageShell eyebrow="Missing" title="Page not found">
      <p>The requested page does not exist.</p>
    </PageShell>
  );
}
