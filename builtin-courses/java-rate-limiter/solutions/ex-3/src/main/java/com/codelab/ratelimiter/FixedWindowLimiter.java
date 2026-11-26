package com.codelab.ratelimiter;

public class FixedWindowLimiter {
    private final int maxRequests;
    private final long windowMillis;
    private final Clock clock;
    private long currentWindow = Long.MIN_VALUE;
    private int count = 0;

    public FixedWindowLimiter(int maxRequests, long windowMillis, Clock clock) {
        if (maxRequests <= 0 || windowMillis <= 0 || clock == null) {
            throw new IllegalArgumentException("invalid fixed window parameters");
        }
        this.maxRequests = maxRequests;
        this.windowMillis = windowMillis;
        this.clock = clock;
    }

    public synchronized RateLimitResult tryAcquireResult() {
        long now = clock.nowMillis();
        long windowIndex = Math.floorDiv(now, windowMillis);
        if (windowIndex != currentWindow) {
            currentWindow = windowIndex;
            count = 0;
        }
        if (count < maxRequests) {
            count++;
            return new RateLimitResult(true, maxRequests - count, 0L);
        }
        long retryAfter = Math.max(1L, (windowIndex + 1) * windowMillis - now);
        return new RateLimitResult(false, 0, retryAfter);
    }
}
