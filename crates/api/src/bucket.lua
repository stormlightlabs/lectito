local key = KEYS[1]
local now = tonumber(ARGV[1])
local capacity = tonumber(ARGV[2])
local refill_per_ms = tonumber(ARGV[3])
local ttl_ms = tonumber(ARGV[4])

local current = redis.call("HMGET", key, "tokens", "updated_at")
local tokens = tonumber(current[1]) or capacity
local updated_at = tonumber(current[2]) or now

if now > updated_at then
    tokens = math.min(capacity, tokens + ((now - updated_at) * refill_per_ms))
end

local allowed = 0
local retry_after = 0

if tokens >= 1 then
    allowed = 1
    tokens = tokens - 1
else
    retry_after = math.ceil((1 - tokens) / refill_per_ms / 1000)
end

redis.call("HSET", key, "tokens", tokens, "updated_at", now)
redis.call("PEXPIRE", key, ttl_ms)

return { allowed, retry_after }
