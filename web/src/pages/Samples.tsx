import { type GallerySample, gallerySamples, sampleCategories, type SampleCategory } from "$lib/samples";
import { useNavigate } from "@solidjs/router";
import { createMemo, createSignal, For, Show } from "solid-js";
import { PageShell } from "./PageShell";
import { WorkbenchTabs } from "./WorkbenchTabs";

function sampleMatches(sample: GallerySample, query: string): boolean {
  if (!query) return true;
  const haystack = `${sample.label} ${sample.description} ${sample.category}`.toLowerCase();
  return haystack.includes(query.toLowerCase());
}

export function SamplesPage() {
  const navigate = useNavigate();
  const [query, setQuery] = createSignal("");

  const grouped = createMemo(() => {
    const filtered = gallerySamples.filter((sample) => sampleMatches(sample, query()));
    const byCategory = new Map<SampleCategory, GallerySample[]>();
    for (const sample of filtered) {
      const list = byCategory.get(sample.category) ?? [];
      list.push(sample);
      byCategory.set(sample.category, list);
    }
    return sampleCategories.map((category) => ({ category, samples: byCategory.get(category) ?? [] })).filter((group) =>
      group.samples.length > 0
    );
  });

  return (
    <PageShell eyebrow="Workbench" title="Sample gallery" headerBefore={<WorkbenchTabs />} variant="workbench">
      <div class="sample-gallery">
        <p class="sample-gallery__intro">
          Curated HTML fixtures exercising specific extraction capabilities. Pick one to load it in the workbench.
        </p>
        <label class="sample-gallery__search">
          <span class="sr-only">Search samples</span>
          <input
            type="search"
            placeholder="Search by name or capability…"
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)} />
        </label>
        <Show when={grouped().length > 0} fallback={<p class="sample-gallery__empty">No samples match.</p>}>
          <For each={grouped()}>
            {(group) => (
              <section class="sample-gallery__group">
                <h2 class="sample-gallery__heading">{group.category}</h2>
                <div class="sample-gallery__cards">
                  <For each={group.samples}>
                    {(sample) => (
                      <button
                        type="button"
                        class="sample-card"
                        onClick={() => navigate(`/workbench?sample=${sample.id}`)}>
                        <span class="sample-card__label">{sample.label}</span>
                        <span class="sample-card__description">{sample.description}</span>
                      </button>
                    )}
                  </For>
                </div>
              </section>
            )}
          </For>
        </Show>
      </div>
    </PageShell>
  );
}
