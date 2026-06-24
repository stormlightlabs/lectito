import { ApiPage } from "$pages/Api";
import { AppLayout } from "$pages/AppLayout";
import { HistoryPage } from "$pages/History";
import { LandingPage } from "$pages/Landing";
import { NotFoundPage } from "$pages/NotFound";
import { RunPage } from "$pages/Run";
import { SamplesPage } from "$pages/Samples";
import { SettingsPage } from "$pages/Settings";
import { WorkbenchPage } from "$pages/Workbench";
import { Route, Router } from "@solidjs/router";

export default function App() {
  return (
    <Router root={AppLayout}>
      <Route path="/" component={LandingPage} />
      <Route path="/workbench" component={WorkbenchPage} />
      <Route path="/history" component={HistoryPage} />
      <Route path="/runs/:id" component={RunPage} />
      <Route path="/samples" component={SamplesPage} />
      <Route path="/api" component={ApiPage} />
      <Route path="/settings" component={SettingsPage} />
      <Route path="*404" component={NotFoundPage} />
    </Router>
  );
}
