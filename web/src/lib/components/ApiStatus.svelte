<script lang="ts">
  import { getApiErrorMessage, getHealth } from '$lib/api';
  import type { HealthResponse } from '$lib/types';
  import { onMount } from 'svelte';

  const POLL_INTERVAL_MS = 15_000;

  type StatusTone = 'checking' | 'operational' | 'degraded' | 'unavailable';

  let health = $state<HealthResponse>();
  let tone = $state<StatusTone>('checking');
  let label = $state('Checking API health…');
  let detail = $state<string>();

  function applyHealthStatus(payload: HealthResponse) {
    health = payload;
    detail = undefined;

    if (payload.status === 'ok' && payload.database === 'ok') {
      tone = 'operational';
      label = 'API operational';
      return;
    }

    if (payload.status === 'degraded' || payload.database !== 'ok') {
      tone = 'degraded';
      label = 'API degraded';
      detail = payload.database === 'unreachable' ? 'Database unreachable' : `Database: ${payload.database}`;
      return;
    }

    tone = 'degraded';
    label = `API ${payload.status}`;
  }

  async function refresh() {
    try {
      const result = await getHealth(fetch);
      applyHealthStatus(result.data);
    } catch (error) {
      health = undefined;
      tone = 'unavailable';
      label = 'API unavailable';
      detail = getApiErrorMessage(error);
    }
  }

  onMount(() => {
    void refresh();

    const interval = globalThis.setInterval(() => {
      void refresh();
    }, POLL_INTERVAL_MS);

    return () => {
      globalThis.clearInterval(interval);
    };
  });
</script>

<div class="flex items-center gap-3 font-mono text-xs text-fog" title={detail ?? undefined}>
  <span class={`site-status-dot h-2.5 w-2.5 rounded-full site-status-dot--${tone}`}></span>
  <span>{label}</span>
  {#if detail}
    <span class="text-stone">({detail})</span>
  {:else if health}
    <span class="text-stone">v{health.version}</span>
  {/if}
</div>
