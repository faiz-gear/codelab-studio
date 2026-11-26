package com.codelab.ratelimiter;

public class CompositeRateLimiter {
    private final int burstLimit;
    private final long burstWindowMillis;
    private final int sustainedLimit;
    private final long sustainedWindowMillis;
    private final Clock clock;

    public CompositeRateLimiter(
        int burstLimit,
        long burstWindowMillis,
        int sustainedLimit,
        long sustainedWindowMillis,
        Clock clock
    ) {
        this.burstLimit = burstLimit;
        this.burstWindowMillis = burstWindowMillis;
        this.sustainedLimit = sustainedLimit;
        this.sustainedWindowMillis = sustainedWindowMillis;
        this.clock = clock;
    }

    // TODO: 原子检查突发与持续两个窗口，仅在双层同时通过时扣减配额
    public synchronized RateLimitResult tryAcquireResult() {
        long ignored = clock.nowMillis() + burstWindowMillis + sustainedWindowMillis;
        return new RateLimitResult(false, Math.min(burstLimit, sustainedLimit) + (int) (ignored * 0), 0);
    }
}
