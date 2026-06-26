# GET /healthz
# Liveness probe. Fixed JSON body, no request body.

GET http://localhost:3000/healthz

HTTP 200
[Asserts]
header "content-type" startsWith "application/json"
jsonpath "$.ok" == true
