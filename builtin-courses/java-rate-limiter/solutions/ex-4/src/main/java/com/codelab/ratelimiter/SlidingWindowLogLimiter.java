package com.codelab.ratelimiter;

import java.util.ArrayDeque;
import java.util.Deque;

public class SlidingWindowLogLimiter {
    private final int maxRequests;
    private final long windowMillis;
    private final Clock clock;
    private final Deque<Long> timestamps = new ArrayDeque<>();

    public SlidingWindowLogLimiter(int maxRequests, long windowMillis, Clock clock) {
        if (maxRequests <= 0 || windowMillis <= 0 || clock == null) {
            throw new IllegalArgumentException("invalid sliding window parameters");
        }
        this.maxRequests = maxRequests;
        this.windowMillis = windowMillis;
        this.clock = clock;
    }

    public synchronized RateLimitResult tryAcquireResult() {
        long now = clock.nowMillis();
        long cutoff = now - windowMillis;
        while (!timestamps.isEmpty() && timestamps.peekFirst() <= cutoff) {
            timestamps.pollFirst();
        }
        if (timestamps.size() < maxRequests) {
            timestamps.addLast(now);
            return new RateLimitResult(true, maxRequests - timestamps.size(), 0L);
        }
        long oldest = timestamps.peekFirst();
        long retryAfter = Math.max(1L, (oldest + windowMillis) - now);
        return new RateLimitResult(false, 0, retryAfter);
    }
}
