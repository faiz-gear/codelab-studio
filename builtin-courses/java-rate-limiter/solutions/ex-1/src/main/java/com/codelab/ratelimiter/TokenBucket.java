package com.codelab.ratelimiter;

public class TokenBucket implements RateLimiter {
    private int tokens;

    public TokenBucket(int capacity) {
        if (capacity <= 0) {
            throw new IllegalArgumentException("capacity must be positive");
        }
        this.tokens = capacity;
    }

    @Override
    public synchronized boolean tryAcquire() {
        if (tokens > 0) {
            tokens--;
            return true;
        }
        return false;
    }

    public synchronized int availableTokens() {
        return tokens;
    }
}
