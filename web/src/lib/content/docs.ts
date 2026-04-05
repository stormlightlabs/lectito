export const DOCS = {
  meta: {
    title: 'API Docs · Lectito',
    description: 'Live OpenAPI reference for the Lectito extraction and library API.'
  },
  hero: {
    label: 'Live reference generated from the running server',
    heading: 'API Docs',
    body: 'The page below reads the embedded OpenAPI document, then turns it into a browsable reference for extraction, library, and operational endpoints.'
  },
  sections: { overview: 'Overview', endpoints: 'Endpoints', schemas: 'Schemas', examples: 'Examples' },
  states: {
    loading: 'Loading the OpenAPI document…',
    empty: 'No operations were published in the current OpenAPI document.',
    errorLabel: 'Docs Error'
  },
  examples: {
    heading: 'Example Usage',
    items: [
      {
        label: 'cURL',
        language: 'bash',
        code: String.raw`curl -X POST http://localhost:3000/api/v1/extract \
  -H "Content-Type: application/json" \
  -d '{"url":"https://example.com/article","format":"markdown"}'`
      },
      {
        label: 'JavaScript',
        language: 'ts',
        code: `const response = await fetch('/api/v1/extract', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    url: 'https://example.com/article',
    format: 'markdown'
  })
});

const result = await response.json();
console.log(result.content);`
      }
    ]
  },
  notes: [
    'Rate-limit headers are exposed on every API response.',
    'The raw OpenAPI document is available at /api-docs/openapi.json.',
    'The server also exposes Swagger UI at /api-docs for a direct explorer view.'
  ]
} as const;

export const LINKS = {
  swagger: { href: '/api-docs', label: 'Open Swagger UI' },
  openApiJson: { href: '/api-docs/openapi.json', label: 'View OpenAPI JSON' }
};
