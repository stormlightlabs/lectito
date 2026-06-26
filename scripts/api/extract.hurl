# POST /v1/extract
# Fetches a URL and returns the extracted article plus optional
# diagnostics.

# Happy path: a stable, article-bearing Wikipedia page.
POST http://localhost:3000/v1/extract
Content-Type: application/json

{
  "url": "https://en.wikipedia.org/wiki/Web_scraping",
  "diagnostics": true
}

HTTP 200
[Asserts]
header "content-type" startsWith "application/json"
jsonpath "$.article" exists
jsonpath "$.article.title" == "Web scraping"
jsonpath "$.article.markdown" isString
jsonpath "$.article.length" isInteger
jsonpath "$.diagnostics" isObject
jsonpath "$.elapsedMs" isInteger

# Structured error: a non-absolute url is rejected by the handler.
POST http://localhost:3000/v1/extract
Content-Type: application/json

{"url": "not-a-url"}

HTTP 400
[Asserts]
header "content-type" startsWith "application/json"
header "x-error-code" == "invalid_request"
jsonpath "$.error.code" == "invalid_request"
jsonpath "$.error.message" isString

# Deserialization error: a missing required field fails the Json
# extractor, which normalizes into a structured ErrorResponse (422
# with JSON body and the x-error-code header) instead of axum's
# default plain-text rejection.
POST http://localhost:3000/v1/extract
Content-Type: application/json

{}

HTTP 422
[Asserts]
header "content-type" startsWith "application/json"
header "x-error-code" == "invalid_request"
jsonpath "$.error.code" == "invalid_request"
jsonpath "$.error.message" isString
