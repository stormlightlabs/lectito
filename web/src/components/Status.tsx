import { createMemo } from "solid-js";
import type { AppMode, PipelineFailure, PipelineResult } from "../lib/types";

function statusText(running: boolean, result?: PipelineResult | PipelineFailure): string {
  if (running) return "Converting";
  if (!result) return "Waiting";
  return "message" in result ? "Error" : "Ready";
}

function elapsedText(result?: PipelineResult | PipelineFailure): string {
  if (!result) return "-";
  return `${result.elapsedMs}ms`;
}

function textLength(result?: PipelineResult | PipelineFailure): string {
  if (!result || "message" in result) return "-";
  return result.metadata.length.toLocaleString();
}

function resultKind(result?: PipelineResult | PipelineFailure): string {
  if (!result) return "-";
  if ("message" in result) return "Fallback";
  return result.mode === "article" ? "Article" : "Fragment";
}

export function StatusItem(props: { label: string; value: string }) {
  return (
    <div>
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </div>
  );
}

type StatusStripProps = { mode: AppMode; running: boolean; result?: PipelineResult | PipelineFailure };

export function StatusStrip(props: StatusStripProps) {
  const result = () => props.result;
  const status = () => statusText(props.running, result());
  const details = createMemo(() =>
    [
      props.mode === "html" ? "Pasted HTML" : "URL / API",
      result() ? resultKind(result()) : "",
      textLength(result()) === "-" ? "" : `${textLength(result())} chars`,
      elapsedText(result()) === "-" ? "" : elapsedText(result()),
    ].filter(Boolean).join(" · ")
  );

  return (
    <section class="status-strip" aria-label="Current conversion status">
      <StatusItem label={status()} value={details()} />
    </section>
  );
}
