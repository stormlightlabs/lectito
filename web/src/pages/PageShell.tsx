import type { JSX, ParentProps } from "solid-js";

type PageShellProps =
  & { eyebrow: string; title: string; headerBefore?: JSX.Element; headerAfter?: JSX.Element }
  & ParentProps;

export function PageShell(props: PageShellProps) {
  return (
    <main class="app-shell">
      <header class="app-header">
        <div>
          {props.headerBefore}
          <p class="eyebrow">{props.eyebrow}</p>
          <h1>{props.title}</h1>
          {props.headerAfter}
        </div>
      </header>
      <section class="pane">
        <div class="empty">{props.children}</div>
      </section>
    </main>
  );
}
