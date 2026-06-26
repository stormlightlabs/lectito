import type { JSX, ParentProps } from "solid-js";

type PageShellProps = {
  eyebrow: string;
  title: string;
  headerBefore?: JSX.Element;
  headerAfter?: JSX.Element;
  variant?: "default" | "workbench";
} & ParentProps;

export function PageShell(props: PageShellProps) {
  const isWorkbench = () => props.variant === "workbench";
  return (
    <main classList={{ "app-shell": true, "app-shell--workbench": isWorkbench() }}>
      {isWorkbench() && props.headerBefore}
      <header classList={{ "app-header": true, "app-header--workbench": isWorkbench() }}>
        <div classList={{ "app-header__main": isWorkbench() }}>
          <div>
            {!isWorkbench() && props.headerBefore}
            <p class="eyebrow">{props.eyebrow} | {props.title}</p>
          </div>
          {props.headerAfter}
        </div>
      </header>
      <section classList={{ "pane": true, "workbench-page": isWorkbench() }}>
        <div class="empty">{props.children}</div>
      </section>
    </main>
  );
}
