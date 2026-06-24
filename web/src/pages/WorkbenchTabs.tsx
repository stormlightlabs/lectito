import { A, useLocation } from "@solidjs/router";
import { For } from "solid-js";

const tabs = [
  { href: "/workbench", label: "Extract", match: (path: string) => path === "/workbench" },
  { href: "/workbench/runs", label: "Runs", match: (path: string) => path.startsWith("/workbench/runs") },
  { href: "/workbench/samples", label: "Samples", match: (path: string) => path.startsWith("/workbench/samples") },
  { href: "/workbench/settings", label: "Settings", match: (path: string) => path.startsWith("/workbench/settings") },
] as const;

export function WorkbenchTabs() {
  const location = useLocation();

  return (
    <nav class="workbench-tabs" role="tablist" aria-label="Workbench sections">
      <For each={tabs}>
        {(tab) => {
          const active = () => tab.match(location.pathname);

          return (
            <A
              href={tab.href}
              role="tab"
              aria-selected={active()}
              classList={{ "is-active": active() }}>
              {tab.label}
            </A>
          );
        }}
      </For>
    </nav>
  );
}
