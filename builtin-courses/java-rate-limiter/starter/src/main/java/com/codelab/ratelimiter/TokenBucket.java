package com.codelab.ratelimiter;

public class TokenBucket implements RateLimiter {
    private int tokens;

    public TokenBucket(int capacity) {
        // TODO: 校验 capacity > 0
        this.tokens = capacity;
    }

    // TODO: 修正边界条件并添加同步控制防止多线程超发
    @Override
    public boolean tryAcquire() {
        if (tokens >= 0) {
            tokens--;
            return true;
        }
        return false;
    }

    public int availableTokens() {
        return tokens;
    }
}
