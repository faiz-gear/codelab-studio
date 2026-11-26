package com.codelab.ratelimiter;

import java.util.function.Supplier;

public class TenantRateLimiter {
    private final Supplier<RateLimiter> factory;

    public TenantRateLimiter(Supplier<RateLimiter> factory) {
        this.factory = factory;
    }

    // TODO: 校验非空租户标识并使用 ConcurrentHashMap.computeIfAbsent 隔离各租户配额
    public boolean tryAcquire(String tenantId) {
        if (factory == null) {
            return false;
        }
        return false;
    }

    public int trackedTenantCount() {
        return 0;
    }
}
