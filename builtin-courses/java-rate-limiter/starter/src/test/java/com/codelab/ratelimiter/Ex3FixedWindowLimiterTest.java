package com.codelab.ratelimiter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

class Ex3FixedWindowLimiterTest {
    @Test
    void enforcesLimitWithinSingleWindow() {
        AtomicLong now = new AtomicLong(1_000L);
        FixedWindowLimiter limiter = new FixedWindowLimiter(2, 1_000L, now::get);

        assertEquals(new RateLimitResult(true, 1, 0L), limiter.tryAcquireResult());
        assertEquals(new RateLimitResult(true, 0, 0L), limiter.tryAcquireResult());
        assertFalse(limiter.tryAcquireResult().allowed());
    }

    @Test
    void resetsQuotaAtNextWindowBoundary() {
        AtomicLong now = new AtomicLong(1_200L);
        FixedWindowLimiter limiter = new FixedWindowLimiter(1, 1_000L, now::get);

        assertTrue(limiter.tryAcquireResult().allowed());
        assertFalse(limiter.tryAcquireResult().allowed());

        now.set(2_000L);
        assertEquals(new RateLimitResult(true, 0, 0L), limiter.tryAcquireResult());
    }

    @Test
    void returnsExactRetryAfterUntilWindowReset() {
        AtomicLong now = new AtomicLong(1_350L);
        FixedWindowLimiter limiter = new FixedWindowLimiter(1, 1_000L, now::get);

        assertTrue(limiter.tryAcquireResult().allowed());
        RateLimitResult rejected = limiter.tryAcquireResult();
        assertFalse(rejected.allowed());
        assertEquals(650L, rejected.retryAfterMillis());
    }
}
