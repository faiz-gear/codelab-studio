package com.codelab.ratelimiter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

class Ex5TenantRateLimiterTest {
    private static RateLimiter atomicQuota(int quota) {
        AtomicInteger left = new AtomicInteger(quota);
        return () -> left.getAndUpdate(v -> v > 0 ? v - 1 : 0) > 0;
    }

    @Test
    void isolatesQuotasBetweenDistinctTenants() {
        TenantRateLimiter limiter = new TenantRateLimiter(() -> atomicQuota(2));

        assertTrue(limiter.tryAcquire("tenant-a"));
        assertTrue(limiter.tryAcquire(" tenant-a "));
        assertFalse(limiter.tryAcquire("tenant-a"));

        assertTrue(limiter.tryAcquire("tenant-b"));
        assertEquals(2, limiter.trackedTenantCount());
    }

    @Test
    void rejectsNullOrBlankTenants() {
        TenantRateLimiter limiter = new TenantRateLimiter(() -> atomicQuota(1));
        assertThrows(IllegalArgumentException.class, () -> limiter.tryAcquire(null));
        assertThrows(IllegalArgumentException.class, () -> limiter.tryAcquire("   "));
    }

    @Test
    void safelyInitializesSharedTenantUnderConcurrency() throws InterruptedException {
        AtomicInteger created = new AtomicInteger();
        TenantRateLimiter limiter = new TenantRateLimiter(() -> {
            created.incrementAndGet();
            return atomicQuota(5);
        });
        int threads = 32;
        CountDownLatch ready = new CountDownLatch(threads);
        CountDownLatch start = new CountDownLatch(1);
        CountDownLatch done = new CountDownLatch(threads);
        AtomicInteger allowed = new AtomicInteger();

        try (ExecutorService pool = Executors.newFixedThreadPool(threads)) {
            for (int i = 0; i < threads; i++) {
                pool.submit(() -> {
                    ready.countDown();
                    try {
                        start.await();
                        if (limiter.tryAcquire("shared")) {
                            allowed.incrementAndGet();
                        }
                    } catch (InterruptedException e) {
                        Thread.currentThread().interrupt();
                    } finally {
                        done.countDown();
                    }
                });
            }
            assertTrue(ready.await(5, TimeUnit.SECONDS));
            start.countDown();
            assertTrue(done.await(5, TimeUnit.SECONDS));
        }

        assertEquals(1, created.get());
        assertEquals(5, allowed.get());
        assertEquals(1, limiter.trackedTenantCount());
    }
}
