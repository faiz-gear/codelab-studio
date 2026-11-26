package com.codelab.ratelimiter;

@FunctionalInterface
public interface Clock {
    long nowMillis();
}
