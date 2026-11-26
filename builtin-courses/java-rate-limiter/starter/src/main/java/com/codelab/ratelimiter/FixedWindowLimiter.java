package com.codelab.ratelimiter;

public class FixedWindowLimiter {
    private final int maxRequests;
    private final long windowMillis;
    private final Clock clock;

    public FixedWindowLimiter(int maxRequests, long windowMillis, Clock clock) {
        this.maxRequests = maxRequests;
        this.windowMillis = windowMillis;
        this.clock = clock;
    }

    // TODO: 实现固定窗口计数、跨窗重置与 retryAfterMillis 计算
    public synchronized RateLimitResult tryAcquireResult() {
        long ignored = clock.nowMillis() + windowMillis;
        return new RateLimitResult(false, maxRequests + (int) (ignored * 0), 0);
    }
}
