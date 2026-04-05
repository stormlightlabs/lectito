<script lang="ts">
  import { resolve } from '$app/paths';
  import { getApiErrorMessage, getOpenApiSpec } from '$lib/api';
  import { DOCS, LINKS } from '$lib/content/docs';
  import type {
    OpenApiDocument,
    OpenApiOperation,
    OpenApiParameter,
    OpenApiParameterObject,
    OpenApiPathItem,
    OpenApiRequestBody,
    OpenApiRequestBodyObject,
    OpenApiResponse,
    OpenApiResponseObject,
    OpenApiSchema,
    OpenApiSchemaObject
  } from '$lib/types';
  import { formatNumber } from '$lib/utils';
  import { SvelteMap } from 'svelte/reactivity';

  const METHODS = ['get', 'post', 'put', 'patch', 'delete'] as const;

  type HttpMethod = (typeof METHODS)[number];

  type MethodLabel = Uppercase<HttpMethod>;

  type EndpointResponse = {
    status: string;
    description: string;
    schemaLabel: string | null;
    headers: { name: string; description: string; type: string }[];
  };

  type EndpointRequest = { required: boolean; contentType: string; schemaLabel: string; example: string | null };

  type EndpointDoc = {
    id: string;
    tag: string;
    path: string;
    method: MethodLabel;
    summary: string;
    description: string;
    operationId: string | null;
    parameters: OpenApiParameterObject[];
    request: EndpointRequest | null;
    responses: EndpointResponse[];
  };

  type SchemaCard = {
    name: string;
    summary: string;
    description: string;
    properties: { name: string; summary: string; required: boolean }[];
  };

  let spec = $state<OpenApiDocument | null>(null);
  let errorMessage = $state<string | null>(null);
  let loading = $state(true);

  $effect(() => {
    let cancelled = false;

    void (async () => {
      try {
        const result = await getOpenApiSpec(fetch);
        if (cancelled) return;
        spec = result.data;
        errorMessage = null;
      } catch (error) {
        if (cancelled) return;
        spec = null;
        errorMessage = getApiErrorMessage(error);
      } finally {
        if (!cancelled) loading = false;
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  function isReference(value: unknown): value is { $ref: string } {
    return typeof value === 'object' && value !== null && '$ref' in value;
  }

  function resolveReference<T>(ref: string): T | null {
    if (!spec || !ref.startsWith('#/')) return null;

    const parts = ref.slice(2).split('/');
    let current: unknown = spec;

    for (const part of parts) {
      if (typeof current !== 'object' || current === null || !(part in current)) {
        return null;
      }

      current = current[part as keyof typeof current];
    }

    return current as T;
  }

  function resolveSchema(schema?: OpenApiSchema): OpenApiSchemaObject | null {
    if (!schema) return null;
    return isReference(schema) ? resolveReference<OpenApiSchemaObject>(schema.$ref) : schema;
  }

  function resolveParameter(parameter: OpenApiParameter): OpenApiParameterObject | null {
    return isReference(parameter) ? resolveReference<OpenApiParameterObject>(parameter.$ref) : parameter;
  }

  function resolveRequestBody(requestBody?: OpenApiRequestBody): OpenApiRequestBodyObject | null {
    if (!requestBody) return null;
    return isReference(requestBody) ? resolveReference<OpenApiRequestBodyObject>(requestBody.$ref) : requestBody;
  }

  function resolveResponse(response: OpenApiResponse): OpenApiResponseObject | null {
    return isReference(response) ? resolveReference<OpenApiResponseObject>(response.$ref) : response;
  }

  function schemaNameFromReference(schema?: OpenApiSchema) {
    if (!schema || !isReference(schema)) return null;
    return schema.$ref.split('/').at(-1) ?? null;
  }

  function schemaSummary(schema?: OpenApiSchema): string {
    const refName = schemaNameFromReference(schema);
    if (refName) return refName;

    const resolved = resolveSchema(schema);
    if (!resolved) return 'Unknown';

    if (resolved.enum?.length) {
      return resolved.enum.join(' | ');
    }

    if (Array.isArray(resolved.type)) {
      return resolved.type.join(' | ');
    }

    if (resolved.type === 'array') {
      return `array<${schemaSummary(resolved.items)}>`;
    }

    if (resolved.type) {
      return resolved.format ? `${resolved.type} (${resolved.format})` : resolved.type;
    }

    if (resolved.properties) {
      return 'object';
    }

    return 'schema';
  }

  function schemaExampleValue(schema?: OpenApiSchema): unknown {
    const resolved = resolveSchema(schema);
    if (!resolved) return null;

    if (resolved.example !== undefined) return resolved.example;
    if (resolved.default !== undefined) return resolved.default;
    if (resolved.enum?.length) return resolved.enum[0];
    if (resolved.type === 'string') return resolved.format === 'uri' ? 'https://example.com' : 'string';
    if (resolved.type === 'integer' || resolved.type === 'number') return 0;
    if (resolved.type === 'boolean') return false;
    if (resolved.type === 'array') return [schemaExampleValue(resolved.items)];

    if (resolved.properties) {
      return Object.fromEntries(
        Object.entries(resolved.properties).map(([name, property]) => [name, schemaExampleValue(property)])
      );
    }

    return null;
  }

  function serializeExample(value: unknown) {
    if (value === null || value === undefined) return null;
    return JSON.stringify(value, null, 2);
  }

  function endpointRequest(operation: OpenApiOperation): EndpointRequest | null {
    const requestBody = resolveRequestBody(operation.requestBody);
    if (!requestBody?.content) return null;

    const [contentType, mediaType] =
      Object.entries(requestBody.content).find(([type]) => type === 'application/json') ??
      Object.entries(requestBody.content)[0] ??
      [];

    if (!contentType || !mediaType) return null;

    return {
      required: Boolean(requestBody.required),
      contentType,
      schemaLabel: schemaSummary(mediaType.schema),
      example: serializeExample(mediaType.example ?? schemaExampleValue(mediaType.schema))
    };
  }

  function endpointResponses(operation: OpenApiOperation): EndpointResponse[] {
    return Object.entries(operation.responses)
      .map(([status, response]) => {
        const resolved = resolveResponse(response);
        if (!resolved) return null;

        const mediaType =
          resolved.content?.['application/json'] ?? (resolved.content ? Object.values(resolved.content)[0] : undefined);

        return {
          status,
          description: resolved.description ?? 'No description provided.',
          schemaLabel: mediaType?.schema ? schemaSummary(mediaType.schema) : null,
          headers: Object.entries(resolved.headers ?? {}).map(([name, header]) => ({
            name,
            description: header.description ?? '',
            type: schemaSummary(header.schema)
          }))
        };
      })
      .filter((response): response is EndpointResponse => response !== null);
  }

  function normalizeParameters(pathItem: OpenApiPathItem, operation: OpenApiOperation) {
    const candidates = [...(pathItem.parameters ?? []), ...(operation.parameters ?? [])];
    const params = candidates
      .map((parameter) => resolveParameter(parameter))
      .filter((parameter): parameter is OpenApiParameterObject => parameter !== null);

    return params.filter(
      (parameter, index) =>
        params.findIndex((candidate) => candidate.name === parameter.name && candidate.in === parameter.in) === index
    );
  }

  function methodTone(method: MethodLabel) {
    switch (method) {
      case 'GET': {
        return 'bg-sky-50 text-sky-800 border-sky-200';
      }
      case 'POST': {
        return 'bg-emerald-50 text-emerald-800 border-emerald-200';
      }
      case 'PUT': {
        return 'bg-amber-50 text-amber-900 border-amber-200';
      }
      case 'PATCH': {
        return 'bg-fuchsia-50 text-fuchsia-800 border-fuchsia-200';
      }
      case 'DELETE': {
        return 'bg-rose-50 text-rose-800 border-rose-200';
      }
    }
  }

  function slugify(value: string) {
    return value
      .toLowerCase()
      .replaceAll(/[^a-z0-9]+/g, '-')
      .replaceAll(/^-+|-+$/g, '');
  }

  const endpointGroups = $derived.by(() => {
    if (!spec) return [] as { name: string; description: string; endpoints: EndpointDoc[] }[];

    const groups = new SvelteMap<string, EndpointDoc[]>();

    for (const [path, pathItem] of Object.entries(spec.paths)) {
      for (const method of METHODS) {
        const operation = pathItem[method];
        if (!operation) continue;

        const tag = operation.tags?.[0] ?? 'Other';
        const endpoint: EndpointDoc = {
          id: `${method}-${slugify(path)}`,
          tag,
          path,
          method: method.toUpperCase() as MethodLabel,
          summary: operation.summary ?? path,
          description: operation.description ?? '',
          operationId: operation.operationId ?? null,
          parameters: normalizeParameters(pathItem, operation),
          request: endpointRequest(operation),
          responses: endpointResponses(operation)
        };

        groups.set(tag, [...(groups.get(tag) ?? []), endpoint]);
      }
    }

    const tagDescriptions = new Map((spec.tags ?? []).map((tag) => [tag.name, tag.description ?? '']));

    return [...groups.entries()]
      .map(([name, endpoints]) => ({ name, description: tagDescriptions.get(name) ?? '', endpoints }))
      .toSorted((a, b) => a.name.localeCompare(b.name));
  });

  const schemaCards = $derived.by(() => {
    if (!spec?.components?.schemas) return [] as SchemaCard[];

    return Object.entries(spec.components.schemas)
      .map(([name, schema]) => ({
        name,
        summary: schemaSummary(schema),
        description: schema.description ?? '',
        properties: Object.entries(schema.properties ?? {}).map(([propertyName, propertySchema]) => ({
          name: propertyName,
          summary: schemaSummary(propertySchema),
          required: (schema.required ?? []).includes(propertyName)
        }))
      }))
      .toSorted((a, b) => a.name.localeCompare(b.name));
  });

  const stats = $derived.by(() => ({
    paths: spec ? Object.keys(spec.paths).length : 0,
    operations: endpointGroups.reduce((count, group) => count + group.endpoints.length, 0),
    schemas: schemaCards.length
  }));
</script>

<svelte:head>
  <title>{DOCS.meta.title}</title>
  <meta name="description" content={DOCS.meta.description} />
</svelte:head>

<div class="mx-auto max-w-6xl px-6 py-12">
  <section class="mb-12 grid gap-8 lg:grid-cols-[minmax(0,1fr)_280px]">
    <div class="editorial-panel overflow-hidden p-8 md:p-10">
      <p class="muted-label mb-4">{DOCS.hero.label}</p>
      <h1 class="mb-4 max-w-3xl font-serif text-4xl font-semibold tracking-tight text-ink md:text-5xl">
        {DOCS.hero.heading}
      </h1>
      <p class="max-w-3xl font-serif text-lg leading-relaxed text-stone">
        {DOCS.hero.body}
      </p>

      <div class="mt-8 grid gap-4 sm:grid-cols-3">
        <div class="border border-mist bg-white/70 p-4">
          <p class="muted-label mb-2">Paths</p>
          <p class="text-3xl font-semibold text-ink">{formatNumber(stats.paths)}</p>
        </div>
        <div class="border border-mist bg-white/70 p-4">
          <p class="muted-label mb-2">Operations</p>
          <p class="text-3xl font-semibold text-ink">{formatNumber(stats.operations)}</p>
        </div>
        <div class="border border-mist bg-white/70 p-4">
          <p class="muted-label mb-2">Schemas</p>
          <p class="text-3xl font-semibold text-ink">{formatNumber(stats.schemas)}</p>
        </div>
      </div>
    </div>

    <aside class="editorial-panel p-6">
      <p class="muted-label mb-4">Reference</p>
      <div class="space-y-3 text-sm text-stone">
        <p class="font-serif">
          {spec?.info.title ?? 'Lectito API'}
          {#if spec?.info.version}
            <span class="font-mono text-xs text-fog"> v{spec.info.version}</span>
          {/if}
        </p>
        <a class="block border-b border-mist pb-3 font-serif hover:text-ink" href="#overview">
          {DOCS.sections.overview}
        </a>
        <a class="block border-b border-mist pb-3 font-serif hover:text-ink" href="#endpoints">
          {DOCS.sections.endpoints}
        </a>
        <a class="block border-b border-mist pb-3 font-serif hover:text-ink" href="#schemas">
          {DOCS.sections.schemas}
        </a>
        <a class="block font-serif hover:text-ink" href="#examples">{DOCS.sections.examples}</a>
      </div>
      <div class="mt-6 space-y-3 border-t border-mist pt-6 text-sm">
        <a
          class="btn-ink block px-4 py-3 text-center font-semibold tracking-[0.16em] uppercase"
          // eslint-disable-next-line svelte/no-navigation-without-resolve
          href={LINKS.swagger.href}>
          {LINKS.swagger.label}
        </a>
        <a
          class="block border-b border-stone pb-1 font-medium text-stone hover:border-ink hover:text-ink"
          // eslint-disable-next-line svelte/no-navigation-without-resolve
          href={LINKS.openApiJson.href}>
          {LINKS.openApiJson.label}
        </a>
      </div>
    </aside>
  </section>

  {#if loading}
    <div class="editorial-panel p-8">
      <p class="muted-label mb-3">Loading</p>
      <p class="font-serif text-lg text-stone">{DOCS.states.loading}</p>
    </div>
  {:else if errorMessage}
    <div class="rounded-2xl border border-red-200 bg-white px-6 py-5 shadow-sm">
      <p class="muted-label mb-2 text-red-700">{DOCS.states.errorLabel}</p>
      <p class="font-serif text-sm text-graphite">{errorMessage}</p>
    </div>
  {:else if spec}
    <div class="grid gap-10 lg:grid-cols-[minmax(0,1fr)_320px]">
      <main class="space-y-10">
        <section id="overview" class="editorial-panel p-8">
          <p class="muted-label mb-4">{DOCS.sections.overview}</p>
          <div class="grid gap-6 md:grid-cols-[minmax(0,1.3fr)_minmax(0,0.9fr)]">
            <div>
              <p class="mb-4 font-serif text-lg leading-relaxed text-charcoal">
                {spec.info.description}
              </p>
              <div class="space-y-3">
                {#each DOCS.notes as note (note)}
                  <p class="border-l-2 border-ink pl-4 font-serif text-sm text-stone">{note}</p>
                {/each}
              </div>
            </div>
            <div class="editorial-card p-5">
              <p class="muted-label mb-3">Published Tags</p>
              <div class="space-y-3">
                {#each spec.tags ?? [] as tag (tag.name)}
                  <div class="border-b border-mist pb-3 last:border-b-0 last:pb-0">
                    <p class="font-semibold text-ink">{tag.name}</p>
                    <p class="mt-1 font-serif text-sm text-stone">{tag.description}</p>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </section>

        <section id="endpoints" class="space-y-6">
          <div class="flex items-end justify-between gap-4">
            <div>
              <p class="muted-label mb-3">{DOCS.sections.endpoints}</p>
              <h2 class="font-serif text-3xl font-semibold text-ink">Live Endpoint Reference</h2>
            </div>
            <a
              class="border-b border-stone text-sm font-medium text-stone hover:border-ink hover:text-ink"
              href={resolve('/about#rate-limits')}>
              Rate-limit policy
            </a>
          </div>

          {#if endpointGroups.length}
            {#each endpointGroups as group (group.name)}
              <section class="space-y-4" id={`tag-${slugify(group.name)}`}>
                <div class="flex flex-wrap items-end justify-between gap-3">
                  <div>
                    <h3 class="font-serif text-2xl font-semibold text-ink">{group.name}</h3>
                    {#if group.description}
                      <p class="mt-1 font-serif text-sm text-stone">{group.description}</p>
                    {/if}
                  </div>
                  <span class="font-mono text-xs text-fog">
                    {formatNumber(group.endpoints.length)} operation{group.endpoints.length === 1 ? '' : 's'}
                  </span>
                </div>

                <div class="space-y-5">
                  {#each group.endpoints as endpoint (endpoint.id)}
                    <article class="editorial-panel overflow-hidden p-6">
                      <div class="mb-5 flex flex-wrap items-center gap-3">
                        <span
                          class={`border px-2.5 py-1 font-mono text-xs font-semibold ${methodTone(endpoint.method)}`}>
                          {endpoint.method}
                        </span>
                        <code class="text-base font-semibold text-ink">{endpoint.path}</code>
                        {#if endpoint.operationId}
                          <span class="font-mono text-xs text-fog">{endpoint.operationId}</span>
                        {/if}
                      </div>

                      <h4 class="mb-2 font-serif text-2xl font-semibold text-ink">{endpoint.summary}</h4>
                      {#if endpoint.description}
                        <p class="mb-5 font-serif text-sm leading-relaxed text-stone">{endpoint.description}</p>
                      {/if}

                      {#if endpoint.parameters.length}
                        <div class="mb-5 overflow-x-auto">
                          <table class="w-full text-left text-sm">
                            <thead>
                              <tr class="border-b border-ink">
                                <th class="py-2 font-semibold">Parameter</th>
                                <th class="py-2 font-semibold">In</th>
                                <th class="py-2 font-semibold">Type</th>
                                <th class="py-2 font-semibold">Details</th>
                              </tr>
                            </thead>
                            <tbody>
                              {#each endpoint.parameters as parameter (`${parameter.in}:${parameter.name}`)}
                                <tr class="border-b border-mist last:border-b-0">
                                  <td class="py-3 font-mono text-xs text-ink">
                                    {parameter.name}
                                    {#if parameter.required}
                                      <span class="ml-2 text-[10px] text-accent uppercase">required</span>
                                    {/if}
                                  </td>
                                  <td class="py-3 font-mono text-xs text-fog">{parameter.in}</td>
                                  <td class="py-3 font-mono text-xs text-fog">{schemaSummary(parameter.schema)}</td>
                                  <td class="py-3 font-serif text-sm text-stone">{parameter.description ?? '—'}</td>
                                </tr>
                              {/each}
                            </tbody>
                          </table>
                        </div>
                      {/if}

                      <div class="grid gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
                        <div class="editorial-card p-5">
                          <p class="muted-label mb-3">Request</p>
                          {#if endpoint.request}
                            <div class="mb-3 flex flex-wrap items-center gap-3 text-xs text-fog">
                              <span class="font-mono">{endpoint.request.contentType}</span>
                              <span>{endpoint.request.required ? 'Required body' : 'Optional body'}</span>
                              <span class="font-mono text-ink">{endpoint.request.schemaLabel}</span>
                            </div>
                            {#if endpoint.request.example}
                              <pre class="raw-view text-xs">{endpoint.request.example}</pre>
                            {:else}
                              <p class="font-serif text-sm text-stone">
                                No example payload was published for this body schema.
                              </p>
                            {/if}
                          {:else}
                            <p class="font-serif text-sm text-stone">This operation does not publish a request body.</p>
                          {/if}
                        </div>

                        <div class="editorial-card p-5">
                          <p class="muted-label mb-3">Responses</p>
                          <div class="space-y-4">
                            {#each endpoint.responses as response (response.status)}
                              <div class="border-b border-mist pb-4 last:border-b-0 last:pb-0">
                                <div class="mb-1 flex flex-wrap items-center gap-3">
                                  <span class="font-mono text-xs text-ink">{response.status}</span>
                                  {#if response.schemaLabel}
                                    <span class="font-mono text-xs text-fog">{response.schemaLabel}</span>
                                  {/if}
                                </div>
                                <p class="font-serif text-sm text-stone">{response.description}</p>
                                {#if response.headers.length}
                                  <div class="mt-2 space-y-1">
                                    {#each response.headers as header (header.name)}
                                      <p class="font-mono text-xs text-fog">
                                        {header.name}
                                        <span class="text-stone">· {header.type}</span>
                                      </p>
                                    {/each}
                                  </div>
                                {/if}
                              </div>
                            {/each}
                          </div>
                        </div>
                      </div>
                    </article>
                  {/each}
                </div>
              </section>
            {/each}
          {:else}
            <div class="editorial-panel p-8 text-center">
              <p class="font-serif text-lg text-stone">{DOCS.states.empty}</p>
            </div>
          {/if}
        </section>

        <section id="schemas" class="space-y-6">
          <div>
            <p class="muted-label mb-3">{DOCS.sections.schemas}</p>
            <h2 class="font-serif text-3xl font-semibold text-ink">Schema Quick Reference</h2>
          </div>

          <div class="grid gap-5 md:grid-cols-2">
            {#each schemaCards as schema (schema.name)}
              <article class="editorial-card p-5">
                <div class="mb-4 flex items-center justify-between gap-4">
                  <h3 class="font-serif text-xl font-semibold text-ink">{schema.name}</h3>
                  <span class="font-mono text-xs text-fog">{schema.summary}</span>
                </div>
                {#if schema.description}
                  <p class="mb-4 font-serif text-sm text-stone">{schema.description}</p>
                {/if}
                {#if schema.properties.length}
                  <div class="space-y-2">
                    {#each schema.properties as property (`${schema.name}:${property.name}`)}
                      <div
                        class="flex items-start justify-between gap-4 border-b border-mist pb-2 last:border-b-0 last:pb-0">
                        <div>
                          <p class="font-mono text-xs text-ink">
                            {property.name}
                            {#if property.required}
                              <span class="ml-2 text-[10px] text-accent uppercase">required</span>
                            {/if}
                          </p>
                        </div>
                        <span class="font-mono text-xs text-fog">{property.summary}</span>
                      </div>
                    {/each}
                  </div>
                {:else}
                  <p class="font-serif text-sm text-stone">No object properties were defined on this schema.</p>
                {/if}
              </article>
            {/each}
          </div>
        </section>

        <section id="examples" class="space-y-6">
          <div>
            <p class="muted-label mb-3">{DOCS.sections.examples}</p>
            <h2 class="font-serif text-3xl font-semibold text-ink">{DOCS.examples.heading}</h2>
          </div>

          <div class="grid gap-6 xl:grid-cols-2">
            {#each DOCS.examples.items as example (example.label)}
              <article class="editorial-panel p-6">
                <div class="mb-4 flex items-center justify-between gap-4">
                  <h3 class="font-serif text-xl font-semibold text-ink">{example.label}</h3>
                  <span class="font-mono text-xs text-fog">{example.language}</span>
                </div>
                <pre class="raw-view text-xs">{example.code}</pre>
              </article>
            {/each}
          </div>
        </section>
      </main>

      <aside class="space-y-6">
        <div class="editorial-panel p-6">
          <p class="muted-label mb-4">Jump To Tag</p>
          <div class="space-y-3">
            {#each endpointGroups as group (group.name)}
              <a
                class="block border-b border-mist pb-3 font-serif text-sm text-stone hover:text-ink"
                href={`#tag-${slugify(group.name)}`}>
                {group.name}
              </a>
            {/each}
          </div>
        </div>

        <div class="editorial-panel p-6">
          <p class="muted-label mb-4">Related</p>
          <div class="space-y-3 text-sm">
            <a class="block font-serif text-stone hover:text-ink" href={resolve('/')}>Run an extraction</a>
            <a class="block font-serif text-stone hover:text-ink" href={resolve('/library')}>Browse the library</a>
            <a class="block font-serif text-stone hover:text-ink" href={resolve('/about')}>Read the overview</a>
          </div>
        </div>
      </aside>
    </div>
  {/if}
</div>
