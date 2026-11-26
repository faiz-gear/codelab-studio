package com.codelab.ratelimiter;

import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicInteger;

public class VirtualThreadLoadRunner {
    public record LoadSummary(int totalRequests, int allowedCount, int rejectedCount) {}

    public LoadSummary run(RateLimiter limiter, int totalRequests) {
        if (limiter == null || totalRequests < 0) {
            throw new IllegalArgumentException("invalid load runner arguments");
        }
        if (totalRequests == 0) {
            return new LoadSummary(0, 0, 0);
        }
        CountDownLatch ready = new CountDownLatch(totalRequests);
        CountDownLatch start = new CountDownLatch(1);
        AtomicInteger allowed = new AtomicInteger();
        AtomicInteger rejected = new AtomicInteger();

        try (ExecutorService executor = Executors.newVirtualThreadPerTaskExecutor()) {
            for (int i = 0; i < totalRequests; i++) {
                executor.submit(() -> {
                    ready.countDown();
                    try {
                        start.await();
                    } catch (InterruptedException e) {
                        Thread.currentThread().interrupt();
                        return;
                    }
                    if (limiter.tryAcquire()) {
                        allowed.incrementAndGet();
                    } else {
                        rejected.incrementAndGet();
                    }
                });
            }
            try {
                ready.await();
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                throw new IllegalStateException("interrupted waiting for virtual threads", e);
            }
            start.countDown();
        }

        return new LoadSummary(totalRequests, allowed.get(), rejected.get());
    }
}
