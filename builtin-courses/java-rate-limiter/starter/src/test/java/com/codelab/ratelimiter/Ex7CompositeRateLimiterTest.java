package com.codelab.ratelimiter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

class Ex7CompositeRateLimiterTest {
    @Test
    void enforcesBurstLimitBeforeSustainedLimit() {
        AtomicLong now = new AtomicLong(1_000L);
        CompositeRateLimiter limiter = new CompositeRateLimiter(2, 100L, 5, 1_000L, now::get);

        assertEquals(new RateLimitResult(true, 1, 0L), limiter.tryAcquireResult());
        assertEquals(new RateLimitResult(true, 0, 0L), limiter.tryAcquireResult());
        RateLimitResult burstBlocked = limiter.tryAcquireResult();
        assertFalse(burstBlocked.allowed());
        assertEquals(100L, burstBlocked.retryAfterMillis());
    }

    @Test
    void doesNotConsumeBurstQuotaWhenSustainedLimitRejects() {
        AtomicLong now = new AtomicLong(0L);
        CompositeRateLimiter limiter = new CompositeRateLimiter(2, 100L, 3, 1_000L, now::get);

        assertTrue(limiter.tryAcquireResult().allowed());
        assertTrue(limiter.tryAcquireResult().allowed());
        now.set(200L);
        assertTrue(limiter.tryAcquireResult().allowed());

        // Sustained window is now full (3/3); repeated rejected calls must not pollute burst window.
        now.set(950L);
        assertFalse(limiter.tryAcquireResult().allowed());
        assertFalse(limiter.tryAcquireResult().allowed());

        // At t=1000 the first two sustained timestamps expire; burst window must still allow 2 calls.
        now.set(1_000L);
        assertTrue(limiter.tryAcquireResult().allowed());
        assertTrue(limiter.tryAcquireResult().allowed());
    }

    @Test
    void returnsLargerRetryAfterWhenBothWindowsBlock() {
        AtomicLong now = new AtomicLong(100L);
        CompositeRateLimiter limiter = new CompositeRateLimiter(1, 200L, 1, 1_000L, now::get);

        assertTrue(limiter.tryAcquireResult().allowed());
        now.set(150L);
        RateLimitResult rejected = limiter.tryAcquireResult();
        assertFalse(rejected.allowed());
        assertEquals(950L, rejected.retryAfterMillis());
    }
}
