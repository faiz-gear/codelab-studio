package com.codelab.ratelimiter;

public class VirtualThreadLoadRunner {
    public record LoadSummary(int totalRequests, int allowedCount, int rejectedCount) {}

    // TODO: 使用 Executors.newVirtualThreadPerTaskExecutor() 并发压测 limiter
    public LoadSummary run(RateLimiter limiter, int totalRequests) {
        if (limiter == null) {
            return new LoadSummary(0, 0, 0);
        }
        return new LoadSummary(totalRequests, 0, 0);
    }
}
