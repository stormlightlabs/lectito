# GET /openapi.json
# utoipa-generated OpenAPI document for the service.

GET http://localhost:3000/openapi.json

HTTP 200
[Asserts]
header "content-type" startsWith "application/json"
jsonpath "$.openapi" exists
jsonpath "$.paths" isObject
jsonpath "$.paths['/v1/extract']" exists
jsonpath "$.paths['/v1/evaluate']" exists
jsonpath "$.paths['/v1/transform']" exists
jsonpath "$.paths['/healthz']" exists
