package com.codelab.ratelimiter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

class Ex6VirtualThreadLoadRunnerTest {
    @Test
    void executesAllRequestsOnVirtualThreadsWithoutOverAllocation() {
        AtomicInteger virtualCalls = new AtomicInteger();
        AtomicInteger quota = new AtomicInteger(15);
        RateLimiter limiter = () -> {
            if (Thread.currentThread().isVirtual()) {
                virtualCalls.incrementAndGet();
            }
            return quota.getAndUpdate(v -> v > 0 ? v - 1 : 0) > 0;
        };

        VirtualThreadLoadRunner runner = new VirtualThreadLoadRunner();
        VirtualThreadLoadRunner.LoadSummary summary = runner.run(limiter, 80);

        assertEquals(80, virtualCalls.get());
        assertEquals(80, summary.totalRequests());
        assertEquals(15, summary.allowedCount());
        assertEquals(65, summary.rejectedCount());
    }

    @Test
    void countsAllowedAndRejectedConsistently() {
        VirtualThreadLoadRunner runner = new VirtualThreadLoadRunner();
        VirtualThreadLoadRunner.LoadSummary empty = runner.run(() -> true, 0);
        assertEquals(new VirtualThreadLoadRunner.LoadSummary(0, 0, 0), empty);

        VirtualThreadLoadRunner.LoadSummary allAllow = runner.run(() -> true, 25);
        assertTrue(allAllow.allowedCount() + allAllow.rejectedCount() == 25);
        assertEquals(25, allAllow.allowedCount());
    }

    @Test
    void rejectsNegativeRequestCount() {
        VirtualThreadLoadRunner runner = new VirtualThreadLoadRunner();
        assertThrows(IllegalArgumentException.class, () -> runner.run(() -> true, -1));
        assertThrows(IllegalArgumentException.class, () -> runner.run(null, 10));
    }
}
