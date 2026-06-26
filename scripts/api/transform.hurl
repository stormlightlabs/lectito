# POST /v1/transform
# Converts raw HTML to Markdown. Returns JSON by default; returns
# text/markdown when the client sends Accept: text/markdown.

# JSON response (default).
POST http://localhost:3000/v1/transform
Content-Type: application/json

{"html": "<h1>Hello</h1><p>Body</p>"}

HTTP 200
[Asserts]
header "content-type" startsWith "application/json"
jsonpath "$.markdown" == "# Hello\n\nBody"

# Plain-Markdown response via content negotiation.
POST http://localhost:3000/v1/transform
Content-Type: application/json
Accept: text/markdown

{"html": "<h1>Hello</h1><p>Body</p>"}

HTTP 200
[Asserts]
header "content-type" startsWith "text/markdown"
body == "# Hello\n\nBody"

# Deserialization error: missing required field. The request body
# fails to deserialize into TransformRequest, which the Json
# extractor normalizes into a structured ErrorResponse (422 with
# JSON body and the x-error-code header) instead of axum's default
# plain-text rejection.
POST http://localhost:3000/v1/transform
Content-Type: application/json

{}

HTTP 422
[Asserts]
header "content-type" startsWith "application/json"
header "x-error-code" == "invalid_request"
jsonpath "$.error.code" == "invalid_request"
jsonpath "$.error.message" isString
