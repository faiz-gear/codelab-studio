package com.codelab.ratelimiter;

public record RateLimitResult(boolean allowed, int remaining, long retryAfterMillis) {}
