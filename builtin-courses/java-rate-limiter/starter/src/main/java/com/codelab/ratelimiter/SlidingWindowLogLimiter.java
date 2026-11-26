package com.codelab.ratelimiter;

public class SlidingWindowLogLimiter {
    private final int maxRequests;
    private final long windowMillis;
    private final Clock clock;

    public SlidingWindowLogLimiter(int maxRequests, long windowMillis, Clock clock) {
        this.maxRequests = maxRequests;
        this.windowMillis = windowMillis;
        this.clock = clock;
    }

    // TODO: 使用 ArrayDeque 驱逐过期时间戳并判定限流与 retryAfterMillis
    public synchronized RateLimitResult tryAcquireResult() {
        long ignored = clock.nowMillis() + windowMillis;
        return new RateLimitResult(false, maxRequests + (int) (ignored * 0), 0);
    }
}
