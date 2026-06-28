import { type JSX, Show, splitProps } from "solid-js";
import { Motion, Presence } from "solid-motionone";

const panelTransition = { duration: 0.18, easing: "ease-out" as const };

const buttonTransition = { duration: 0.12, easing: "ease-out" as const };

type MotionButtonProps = JSX.ButtonHTMLAttributes<HTMLButtonElement> & { staticMotion?: boolean };

export function MotionButton(props: MotionButtonProps) {
  const [local, rest] = splitProps(props, ["staticMotion"]);
  return (
    <Motion.button {...rest} press={local.staticMotion ? undefined : { scale: 0.96 }} transition={buttonTransition} />
  );
}

type MotionPanelProps = JSX.HTMLAttributes<HTMLDivElement> & { show?: boolean };

export function MotionReveal(props: MotionPanelProps) {
  const [local, rest] = splitProps(props, ["show", "children"]);
  return (
    <Presence initial={false}>
      <Show when={local.show ?? true}>
        <Motion.div
          {...rest}
          initial={{ opacity: 0, y: -4 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -4 }}
          transition={panelTransition}>
          {local.children}
        </Motion.div>
      </Show>
    </Presence>
  );
}

type MotionInlineProps = JSX.HTMLAttributes<HTMLSpanElement> & { show?: boolean };

export function MotionInlineReveal(props: MotionInlineProps) {
  const [local, rest] = splitProps(props, ["show", "children"]);
  return (
    <Presence initial={false}>
      <Show when={local.show ?? true}>
        <Motion.span
          {...rest}
          initial={{ opacity: 0, x: -4 }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: -4 }}
          transition={panelTransition}>
          {local.children}
        </Motion.span>
      </Show>
    </Presence>
  );
}

type MotionSwapProps = JSX.HTMLAttributes<HTMLDivElement> & { viewKey: string };

export function MotionSwap(props: MotionSwapProps) {
  const [local, rest] = splitProps(props, ["viewKey", "children"]);
  return (
    <Presence initial={false} exitBeforeEnter>
      <Show keyed when={local.viewKey}>
        <Motion.div
          {...rest}
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -4 }}
          transition={panelTransition}>
          {local.children}
        </Motion.div>
      </Show>
    </Presence>
  );
}
