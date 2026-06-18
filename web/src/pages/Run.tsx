import { useParams } from "@solidjs/router";
import { PageShell } from "./PageShell";

export function RunPage() {
  const params = useParams();

  return (
    <PageShell eyebrow="Run" title={`Run ${params.id}`}>
      <p>Inspect a saved extraction result.</p>
    </PageShell>
  );
}
