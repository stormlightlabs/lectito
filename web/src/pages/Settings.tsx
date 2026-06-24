import { PageShell } from "./PageShell";
import { WorkbenchTabs } from "./WorkbenchTabs";

export function SettingsPage() {
  return (
    <PageShell eyebrow="Workbench" title="Web app settings" headerBefore={<WorkbenchTabs />}>
      <p>Configure defaults for output views and saved runs.</p>
    </PageShell>
  );
}
