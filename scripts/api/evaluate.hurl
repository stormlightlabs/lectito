# POST /v1/evaluate
# Fetches a URL and reports whether the page is probably readable.

# Happy path: a Wikipedia article is readable.
POST http://localhost:3000/v1/evaluate
Content-Type: application/json

{"url": "https://en.wikipedia.org/wiki/Web_scraping"}

HTTP 200
[Asserts]
header "content-type" startsWith "application/json"
jsonpath "$.readable" == true

# Structured error: a non-absolute url.
POST http://localhost:3000/v1/evaluate
Content-Type: application/json

{"url": "not-a-url"}

HTTP 400
[Asserts]
header "x-error-code" == "invalid_request"
jsonpath "$.error.code" == "invalid_request"
