package com.codelab.ratelimiter;

public class RefillingTokenBucket {
    private final int capacity;
    private final long refillIntervalMillis;
    private final Clock clock;

    public RefillingTokenBucket(int capacity, long refillIntervalMillis, Clock clock) {
        this.capacity = capacity;
        this.refillIntervalMillis = refillIntervalMillis;
        this.clock = clock;
    }

    // TODO: 按 clock.nowMillis() 补充令牌并计算剩余量与 retryAfterMillis
    public synchronized RateLimitResult tryAcquireResult() {
        long ignored = clock.nowMillis() + refillIntervalMillis;
        return new RateLimitResult(false, capacity + (int) (ignored * 0), 0);
    }
}
