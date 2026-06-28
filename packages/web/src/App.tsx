import { ApiPage } from "$pages/Api";
import { AppLayout } from "$pages/AppLayout";
import { LandingPage } from "$pages/Landing";
import { NotFoundPage } from "$pages/NotFound";
import { RunPage } from "$pages/Run";
import { RunsPage } from "$pages/Runs";
import { SamplesPage } from "$pages/Samples";
import { SettingsPage } from "$pages/Settings";
import { WorkbenchPage } from "$pages/Workbench";
import { Route, Router } from "@solidjs/router";

export default function App() {
  return (
    <Router root={AppLayout}>
      <Route path="/" component={LandingPage} />
      <Route path="/workbench" component={WorkbenchPage} />
      <Route path="/workbench/runs" component={RunsPage} />
      <Route path="/workbench/runs/:id" component={RunPage} />
      <Route path="/workbench/samples" component={SamplesPage} />
      <Route path="/workbench/settings" component={SettingsPage} />
      <Route path="/api-docs" component={ApiPage} />
      <Route path="*404" component={NotFoundPage} />
    </Router>
  );
}
