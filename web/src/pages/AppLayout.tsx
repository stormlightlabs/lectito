import { A, useLocation } from "@solidjs/router";
import { For } from "solid-js";
import type { ParentProps } from "solid-js";

const NAV_ITEMS = [
  { href: "/", label: "Home" },
  { href: "/workbench", label: "Workbench" },
  { href: "/api", label: "API" },
] as const;

export function AppLayout(props: ParentProps) {
  const location = useLocation();
  const isActive = (href: string) => href === "/" ? location.pathname === "/" : location.pathname.startsWith(href);

  return (
    <div class="app-frame">
      <header class="app-nav">
        <div class="landing-nav">
          <A class="landing-brand" href="/" aria-label="Lectito home">
            <span class="landing-brand__mark" aria-hidden="true">L</span>
            <span>Lectito</span>
          </A>
          <nav class="landing-nav__links" aria-label="Primary">
            <For each={NAV_ITEMS}>
              {(item) => <A href={item.href} classList={{ "is-active": isActive(item.href) }}>{item.label}</A>}
            </For>
          </nav>
        </div>
      </header>

      <div class="app-frame__content">{props.children}</div>

      <footer class="app-footer">
        <div class="app-footer__inner">
          <span>Lectito</span>
          <nav aria-label="Footer">
            <A href="/workbench">Workbench</A>
            <A href="/api">API</A>
          </nav>
        </div>
      </footer>
    </div>
  );
}
