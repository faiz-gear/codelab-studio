package com.codelab.ratelimiter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.atomic.AtomicLong;
import org.junit.jupiter.api.Test;

class Ex2RefillingTokenBucketTest {
    @Test
    void consumesBurstThenRefillsIncrementally() {
        AtomicLong now = new AtomicLong(1_000L);
        RefillingTokenBucket bucket = new RefillingTokenBucket(2, 500L, now::get);

        assertEquals(new RateLimitResult(true, 1, 0L), bucket.tryAcquireResult());
        assertEquals(new RateLimitResult(true, 0, 0L), bucket.tryAcquireResult());
        assertFalse(bucket.tryAcquireResult().allowed());

        now.addAndGet(500L);
        assertEquals(new RateLimitResult(true, 0, 0L), bucket.tryAcquireResult());
    }

    @Test
    void refillsTokensAsClockAdvancesWithoutExceedingCapacity() {
        AtomicLong now = new AtomicLong(0L);
        RefillingTokenBucket bucket = new RefillingTokenBucket(3, 200L, now::get);

        assertTrue(bucket.tryAcquireResult().allowed());
        now.addAndGet(10_000L);
        RateLimitResult result = bucket.tryAcquireResult();
        assertTrue(result.allowed());
        assertEquals(2, result.remaining());
    }

    @Test
    void reportsRetryAfterMillisWhenEmpty() {
        AtomicLong now = new AtomicLong(1_000L);
        RefillingTokenBucket bucket = new RefillingTokenBucket(1, 400L, now::get);

        assertTrue(bucket.tryAcquireResult().allowed());
        now.addAndGet(150L);
        RateLimitResult rejected = bucket.tryAcquireResult();
        assertFalse(rejected.allowed());
        assertEquals(0, rejected.remaining());
        assertEquals(250L, rejected.retryAfterMillis());
    }
}
