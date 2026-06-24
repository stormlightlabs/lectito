import { PageShell } from "./PageShell";
import { WorkbenchTabs } from "./WorkbenchTabs";

export function SamplesPage() {
  return (
    <PageShell eyebrow="Workbench" title="Sample gallery" headerBefore={<WorkbenchTabs />}>
      <p>Browse fixtures and known article extraction cases.</p>
    </PageShell>
  );
}
