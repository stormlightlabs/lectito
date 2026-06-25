import { Match, Switch } from "solid-js";

export type IconKind =
  | "cancel"
  | "collapseInput"
  | "convert"
  | "copy"
  | "download"
  | "exitFullscreen"
  | "fullscreen"
  | "html"
  | "inspect"
  | "markdown"
  | "metadata"
  | "more"
  | "open"
  | "preview"
  | "reset"
  | "save"
  | "share";

type IconProps = { kind: IconKind };

export function Icon(props: IconProps) {
  return (
    <Switch>
      <Match when={props.kind === "cancel"}>
        <span class="icon i-ph-x" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "collapseInput"}>
        <span class="icon i-ph-sidebar-simple" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "convert"}>
        <span class="icon i-ph-play" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "copy"}>
        <span class="icon i-ph-copy" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "download"}>
        <span class="icon i-ph-download-simple" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "exitFullscreen"}>
        <span class="icon i-ph-corners-in" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "fullscreen"}>
        <span class="icon i-ph-corners-out" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "html"}>
        <span class="icon i-ph-file-html" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "inspect"}>
        <span class="icon i-ph-list-magnifying-glass" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "markdown"}>
        <span class="icon i-ph-markdown-logo" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "metadata"}>
        <span class="icon i-ph-brackets-curly" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "more"}>
        <span class="icon i-ph-dots-three-vertical" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "open"}>
        <span class="icon i-ph-arrow-square-out" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "preview"}>
        <span class="icon i-ph-eye" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "reset"}>
        <span class="icon i-ph-arrow-counter-clockwise" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "save"}>
        <span class="icon i-ph-floppy-disk" aria-hidden="true" />
      </Match>
      <Match when={props.kind === "share"}>
        <span class="icon i-ph-link-simple" aria-hidden="true" />
      </Match>
    </Switch>
  );
}
