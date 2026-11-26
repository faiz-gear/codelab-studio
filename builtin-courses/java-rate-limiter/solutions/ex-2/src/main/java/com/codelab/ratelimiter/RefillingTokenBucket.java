package com.codelab.ratelimiter;

public class RefillingTokenBucket {
    private final int capacity;
    private final long refillIntervalMillis;
    private final Clock clock;
    private int tokens;
    private long lastRefillMillis;

    public RefillingTokenBucket(int capacity, long refillIntervalMillis, Clock clock) {
        if (capacity <= 0 || refillIntervalMillis <= 0 || clock == null) {
            throw new IllegalArgumentException("invalid token bucket parameters");
        }
        this.capacity = capacity;
        this.refillIntervalMillis = refillIntervalMillis;
        this.clock = clock;
        this.tokens = capacity;
        this.lastRefillMillis = clock.nowMillis();
    }

    public synchronized RateLimitResult tryAcquireResult() {
        long now = clock.nowMillis();
        if (now > lastRefillMillis) {
            long elapsed = now - lastRefillMillis;
            long added = elapsed / refillIntervalMillis;
            if (added > 0) {
                tokens = (int) Math.min(capacity, tokens + added);
                lastRefillMillis = tokens == capacity ? now : lastRefillMillis + added * refillIntervalMillis;
            }
        }
        if (tokens > 0) {
            if (tokens == capacity) {
                lastRefillMillis = now;
            }
            tokens--;
            return new RateLimitResult(true, tokens, 0L);
        }
        long elapsedSinceRefill = Math.max(0L, now - lastRefillMillis);
        long retryAfter = Math.max(1L, refillIntervalMillis - elapsedSinceRefill);
        return new RateLimitResult(false, 0, retryAfter);
    }
}
