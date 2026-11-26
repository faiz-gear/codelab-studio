package com.codelab.ratelimiter;

import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Supplier;

public class TenantRateLimiter {
    private final Supplier<RateLimiter> factory;
    private final ConcurrentHashMap<String, RateLimiter> limiters = new ConcurrentHashMap<>();

    public TenantRateLimiter(Supplier<RateLimiter> factory) {
        if (factory == null) {
            throw new IllegalArgumentException("factory must not be null");
        }
        this.factory = factory;
    }

    public boolean tryAcquire(String tenantId) {
        if (tenantId == null || tenantId.trim().isEmpty()) {
            throw new IllegalArgumentException("tenantId must not be blank");
        }
        String normalized = tenantId.trim();
        RateLimiter limiter = limiters.computeIfAbsent(normalized, ignored -> factory.get());
        return limiter.tryAcquire();
    }

    public int trackedTenantCount() {
        return limiters.size();
    }
}
