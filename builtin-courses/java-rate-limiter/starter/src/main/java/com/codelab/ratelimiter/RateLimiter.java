package com.codelab.ratelimiter;

public interface RateLimiter {
    boolean tryAcquire();
}
