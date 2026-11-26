package com.codelab.ratelimiter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

class Ex4SlidingWindowLogLimiterTest {
    @Test
    void preventsBoundaryBurstAcrossFixedWindowEdges() {
        AtomicLong now = new AtomicLong(900L);
        SlidingWindowLogLimiter limiter = new SlidingWindowLogLimiter(2, 1_000L, now::get);

        assertTrue(limiter.tryAcquireResult().allowed());
        now.set(950L);
        assertTrue(limiter.tryAcquireResult().allowed());
        now.set(1_050L);
        assertFalse(limiter.tryAcquireResult().allowed());
    }

    @Test
    void evictsExpiredTimestampsAndComputesRetryAfter() {
        AtomicLong now = new AtomicLong(1_000L);
        SlidingWindowLogLimiter limiter = new SlidingWindowLogLimiter(2, 500L, now::get);

        assertTrue(limiter.tryAcquireResult().allowed());
        now.set(1_200L);
        assertTrue(limiter.tryAcquireResult().allowed());
        now.set(1_300L);

        RateLimitResult rejected = limiter.tryAcquireResult();
        assertFalse(rejected.allowed());
        assertEquals(200L, rejected.retryAfterMillis());

        now.set(1_500L);
        assertTrue(limiter.tryAcquireResult().allowed());
    }

    @Test
    void tracksRemainingCapacityAccurately() {
        AtomicLong now = new AtomicLong(0L);
        SlidingWindowLogLimiter limiter = new SlidingWindowLogLimiter(3, 1_000L, now::get);

        assertEquals(new RateLimitResult(true, 2, 0L), limiter.tryAcquireResult());
        assertEquals(new RateLimitResult(true, 1, 0L), limiter.tryAcquireResult());
        assertEquals(new RateLimitResult(true, 0, 0L), limiter.tryAcquireResult());
    }
}
