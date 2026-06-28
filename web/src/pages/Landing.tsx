import { Icon } from "$components/Icon";
import { Trans } from "@lingui/solid/macro";
import { A } from "@solidjs/router";
import type { ParentProps } from "solid-js";

function ExtractionDemo() {
  return (
    <div class="landing-demo" aria-label="Extraction before and after">
      <div class="landing-demo__pane landing-demo__pane--source">
        <p class="landing-demo__label">
          <Trans>Original</Trans>
        </p>
        <div class="landing-demo__block is-noise">
          <Trans>cookie banner</Trans>
        </div>
        <div class="landing-demo__block is-noise">
          <Trans>site navigation</Trans>
        </div>
        <div class="landing-demo__split">
          <div class="landing-demo__block is-article">
            <Trans>article body</Trans>
            <span />
            <span class="short" />
          </div>
          <div class="landing-demo__block is-noise">
            <Trans>related links</Trans>
          </div>
        </div>
        <div class="landing-demo__block is-noise">
          <Trans>ad slot</Trans>
        </div>
        <div class="landing-demo__block is-noise">
          <Trans>footer links</Trans>
        </div>
      </div>
      <div class="landing-demo__arrow" aria-hidden="true">
        <Icon kind="arrow-right" />
      </div>
      <div class="landing-demo__pane landing-demo__pane--result">
        <p class="landing-demo__label">
          <Trans>Extracted</Trans>
        </p>
        <h2>
          <Trans>On Reading the Web</Trans>
        </h2>
        <p class="landing-demo__reader-copy">
          <Trans>The article body remains. The page chrome is gone.</Trans>
        </p>
        <span class="landing-demo__reader-line" />
        <span class="landing-demo__reader-line" />
        <span class="landing-demo__reader-line short" />
      </div>
    </div>
  );
}

function CapabilityCard(props: ParentProps<{ number: string }>) {
  return (
    <article class="landing-capability">
      <span>{props.number}</span>
      {props.children}
    </article>
  );
}

function CapabilityGrid() {
  // TODO: this could be an array
  return (
    <div class="landing-capabilities__grid">
      <CapabilityCard number="01">
        <h3>
          <Trans>Add a page</Trans>
        </h3>
        <p>
          <Trans>Paste HTML or enter a URL. Lectito finds the article and removes the surrounding chrome.</Trans>
        </p>
      </CapabilityCard>
      <CapabilityCard number="02">
        <h3>
          <Trans>Read the result</Trans>
        </h3>
        <p>
          <Trans>Preview the cleaned article before you copy anything out of the workbench.</Trans>
        </p>
      </CapabilityCard>
      <CapabilityCard number="03">
        <h3>
          <Trans>Copy what you need</Trans>
        </h3>
        <p>
          <Trans>Use Markdown, clean HTML, metadata, or diagnostics from the same run.</Trans>
        </p>
      </CapabilityCard>
    </div>
  );
}

function HeroActions() {
  return (
    <div class="landing-hero__actions">
      <A class="button button--primary" href="/workbench">
        <Trans>Open the workbench</Trans>
      </A>
      <a class="button button--secondary" href="/docs/" rel="external">
        <Trans>Read the Docs</Trans>
      </a>
    </div>
  );
}

export function LandingPage() {
  return (
    <main class="landing-page">
      <section class="landing-hero">
        <div class="landing-hero__copy">
          <p class="eyebrow">
            <Trans>Article extraction</Trans>
          </p>
          <h1>
            <Trans>Focus on what matters.</Trans>
          </h1>
          <p class="landing-hero__lede">
            <Trans>Lectito is a tool that removes cruft to make the web more readable.</Trans>
          </p>
          <HeroActions />
        </div>

        <ExtractionDemo />
      </section>

      <section class="landing-capabilities" aria-labelledby="landing-capabilities-title">
        <div>
          <p class="eyebrow">
            <Trans>Workbench</Trans>
          </p>
          <h2 id="landing-capabilities-title">
            <Trans>Clean up the document before you read it.</Trans>
          </h2>
        </div>
        <CapabilityGrid />
      </section>
    </main>
  );
}
