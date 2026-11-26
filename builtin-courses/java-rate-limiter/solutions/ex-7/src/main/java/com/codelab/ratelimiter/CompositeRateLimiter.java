package com.codelab.ratelimiter;

import java.util.ArrayDeque;
import java.util.Deque;

public class CompositeRateLimiter {
    private final int burstLimit;
    private final long burstWindowMillis;
    private final int sustainedLimit;
    private final long sustainedWindowMillis;
    private final Clock clock;
    private final Deque<Long> burstLog = new ArrayDeque<>();
    private final Deque<Long> sustainedLog = new ArrayDeque<>();

    public CompositeRateLimiter(
        int burstLimit,
        long burstWindowMillis,
        int sustainedLimit,
        long sustainedWindowMillis,
        Clock clock
    ) {
        if (burstLimit <= 0
            || burstWindowMillis <= 0
            || sustainedLimit <= 0
            || sustainedWindowMillis <= 0
            || clock == null) {
            throw new IllegalArgumentException("invalid composite rate limiter parameters");
        }
        this.burstLimit = burstLimit;
        this.burstWindowMillis = burstWindowMillis;
        this.sustainedLimit = sustainedLimit;
        this.sustainedWindowMillis = sustainedWindowMillis;
        this.clock = clock;
    }

    public synchronized RateLimitResult tryAcquireResult() {
        long now = clock.nowMillis();
        evict(burstLog, now - burstWindowMillis);
        evict(sustainedLog, now - sustainedWindowMillis);

        boolean burstBlocked = burstLog.size() >= burstLimit;
        boolean sustainedBlocked = sustainedLog.size() >= sustainedLimit;
        if (burstBlocked || sustainedBlocked) {
            long burstRetry = burstBlocked ? Math.max(1L, (burstLog.peekFirst() + burstWindowMillis) - now) : 0L;
            long sustainedRetry = sustainedBlocked ? Math.max(1L, (sustainedLog.peekFirst() + sustainedWindowMillis) - now) : 0L;
            return new RateLimitResult(false, 0, Math.max(burstRetry, sustainedRetry));
        }

        burstLog.addLast(now);
        sustainedLog.addLast(now);
        int remaining = Math.min(burstLimit - burstLog.size(), sustainedLimit - sustainedLog.size());
        return new RateLimitResult(true, remaining, 0L);
    }

    private static void evict(Deque<Long> log, long cutoff) {
        while (!log.isEmpty() && log.peekFirst() <= cutoff) {
            log.pollFirst();
        }
    }
}
