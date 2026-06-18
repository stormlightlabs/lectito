import type { ParentProps } from "solid-js";

type PageShellProps = { eyebrow: string; title: string } & ParentProps;

export function PageShell(props: PageShellProps) {
  return (
    <main class="app-shell">
      <header class="app-header">
        <div>
          <p class="eyebrow">{props.eyebrow}</p>
          <h1>{props.title}</h1>
        </div>
      </header>
      <section class="pane">
        <div class="empty">{props.children}</div>
      </section>
    </main>
  );
}
