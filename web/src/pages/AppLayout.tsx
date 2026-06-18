import { MotionButton, MotionInlineReveal } from "$components/shared/Motion";
import { A } from "@solidjs/router";
import { createSignal, For } from "solid-js";
import type { ParentProps } from "solid-js";

const navItems = [
  { href: "/workbench", label: "Workbench" },
  { href: "/history", label: "History" },
  { href: "/samples", label: "Samples" },
  { href: "/api", label: "API" },
  { href: "/settings", label: "Settings" },
] as const;

export function AppLayout(props: ParentProps) {
  const [collapsed, setCollapsed] = createSignal(false);

  return (
    <div class="app-frame" classList={{ "is-sidebar-collapsed": collapsed() }}>
      <aside class="app-sidebar" aria-label="Lectito navigation">
        <div class="app-sidebar__brand">
          <span class="app-sidebar__mark" aria-hidden="true">L</span>
          <MotionInlineReveal show={!collapsed()} class="app-sidebar__label">Lectito</MotionInlineReveal>
        </div>

        <nav class="app-sidebar__nav">
          <For each={navItems}>
            {(item) => (
              <A href={item.href} activeClass="is-active" class="app-sidebar__link">
                <span class="app-sidebar__mark" aria-hidden="true">{item.label.slice(0, 1)}</span>
                <MotionInlineReveal show={!collapsed()} class="app-sidebar__label">{item.label}</MotionInlineReveal>
              </A>
            )}
          </For>
        </nav>

        <MotionButton
          type="button"
          class="app-sidebar__toggle"
          aria-expanded={!collapsed()}
          onClick={() => setCollapsed((value) => !value)}>
          <span class="app-sidebar__mark" aria-hidden="true">{collapsed() ? ">" : "<"}</span>
          <MotionInlineReveal show={!collapsed()} class="app-sidebar__label">Collapse</MotionInlineReveal>
        </MotionButton>
      </aside>

      <div class="app-frame__content">{props.children}</div>
    </div>
  );
}
