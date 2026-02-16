# Failure Mode Analysis

**Project:** Peilbeheer HHVR - Digital Twin Waterbeheer
**Version:** 0.1.0
**Date:** 2026-02-16

---

## Overview

This document identifies potential failure modes, their effects, and mitigation strategies. It follows a systematic approach to understanding how the system can fail and ensuring appropriate safeguards are in place.

---

## Failure Mode Categories

1. **External Dependency Failures** - Third-party service issues
2. **Data Integrity Failures** - Corruption or invalid data
3. **Infrastructure Failures** - Hardware/network problems
4. **Software Failures** - Bugs, crashes, deadlocks
5. **Operational Failures** - Human error, misconfiguration

---

## External Dependency Failures

### EDF-001: ArcGIS API Unavailable

| Aspect | Details |
|--------|---------|
| **Failure Mode** | ArcGIS server down, network issues, or API changes |
| **Detection** | HTTP timeout, connection refused, non-2xx response |
| **Impact** | Medium - Cannot fetch/update asset registrations |
| **Affected Features** | Gemalen list, asset map, peilgebieden display |
| **Current Mitigation** | Cache in DuckDB, serve stale data |
| **Recovery Time** | Manual (until cache refreshes) |
| **Recommended** | Circuit breaker, retry with exponential backoff, health endpoint |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Implement circuit breaker | High | 1 day |
| Add stale-while-revalidate headers | Medium | 2 hours |
| Health check endpoint | High | 1 hour |
| Alert on failure | Medium | 2 hours |

---

### EDF-002: Hydronet API Unavailable

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Hydronet server down or timeout |
| **Detection** | Request timeout > 30s, connection errors |
| **Impact** | High - No real-time gemaal status updates |
| **Affected Features** | Status dashboard, real-time monitoring |
| **Current Mitigation** | Cache last known status in database |
| **Recovery Time** | Automatic when API returns |
| **Recommended** | Queue requests, batch processing, fallback to cache |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Request queue with retry | High | 1 day |
| Graceful degradation message | Medium | 2 hours |
| Cached data freshness indicator | Low | 2 hours |

---

### EDF-003: EnergyZero API Unavailable

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Energy price API down |
| **Detection** | Request timeout, parse errors |
| **Impact** | Medium - No cost-based optimization |
| **Affected Features** | Optimalisatie endpoint |
| **Current Mitigation** | None - optimization will fail |
| **Recovery Time** | Manual intervention |
| **Recommended** | Fallback to time-based schedule |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Default price schedule | High | 4 hours |
| Cache with 24h TTL | Medium | 2 hours |
| Error message to user | High | 1 hour |

---

## Data Integrity Failures

### DIF-001: DuckDB File Corruption

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Database file corrupted, invalid schema |
| **Detection** | Query errors, "database disk image is malformed" |
| **Impact** | Critical - Complete system outage |
| **Affected Features** | All features requiring data |
| **Current Mitigation** | None |
| **Recovery Time** | Manual restore from backup (if exists) |
| **Recommended** | Daily backups, automatic recovery |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Automated daily backups | Critical | 1 day |
| Backup restoration script | Critical | 4 hours |
| Health check on startup | High | 2 hours |
| Multiple backup retention | Medium | 2 hours |

---

### DIF-002: Invalid Geometry Data

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Invalid GeoJSON, self-intersecting polygons |
| **Detection** | DuckDB spatial errors, rendering failures |
| **Impact** | Low - Individual features fail to display |
| **Affected Features** | Map visualization, spatial queries |
| **Current Mitigation** | None - errors may crash queries |
| **Recovery Time** | Manual data fix |
| **Recommended** | Validation on insert, graceful skip |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| ST_IsValid check on insert | High | 4 hours |
| Skip invalid features with log | Medium | 2 hours |
| Geometry repair function | Low | 1 day |

---

### DIF-003: Stale Cache Data

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Auto-sync fails, cache becomes stale |
| **Detection** | `fetched_at` timestamp check |
| **Impact** | Medium - Decisions based on outdated data |
| **Affected Features** | All cached data displays |
| **Current Mitigation** | None - no freshness indicators |
| **Recovery Time** | Manual sync trigger |
| **Recommended** | Freshness display, auto-refresh |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Show "last updated" timestamp | High | 2 hours |
| Auto-refresh button | Medium | 2 hours |
| Background refresh timer | Low | 4 hours |

---

## Infrastructure Failures

### IF-001: Disk Space Exhaustion

| Aspect | Details |
|--------|---------|
| **Failure Mode** | No space for database writes |
| **Detection** | OS error "No space left on device" |
| **Impact** | Critical - Cannot write new data |
| **Affected Features** | All write operations |
| **Current Mitigation** | None |
| **Recovery Time** | Manual cleanup |
| **Recommended** | Disk space monitoring, auto-cleanup |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Disk space alerting | High | 2 hours |
| Auto-delete old data | Medium | 4 hours |
| Disk space requirement in docs | Low | 1 hour |

---

### IF-002: Network Partition

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Cannot reach external APIs |
| **Detection** | Connection timeouts |
| **Impact** | High - No external data updates |
| **Affected Features** | ArcGIS, Hydronet, EnergyZero integrations |
| **Current Mitigation** | Serve from cache |
| **Recovery Time** | Automatic when network returns |
| **Recommended** | Offline mode indicator, queueing |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Offline mode indicator | Medium | 2 hours |
| Request queue for retry | High | 1 day |
| Extended cache retention | Low | 2 hours |

---

### IF-003: Memory Exhaustion

| Aspect | Details |
|--------|---------|
| **Failure Mode** | OOM kill, large query result sets |
| **Detection** | Process killed, slow response |
| **Impact** | Critical - Service restart |
| **Affected Features** | All features |
| **Current Mitigation** | None |
| **Recovery Time** | Automatic restart |
| **Recommended** | Memory limits, query pagination |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Result set size limits | High | 4 hours |
| Pagination for large queries | Medium | 1 day |
| Memory monitoring | Medium | 2 hours |

---

## Software Failures

### SF-001: Unhandled Exceptions

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Panic, unwrap(), unexpected error |
| **Detection** | Process crash, 500 errors |
| **Impact** | Variable - Request or system failure |
| **Affected Features** | Any feature with unhandled error |
| **Current Mitigation** | Partial - `anyhow::Result` in some places |
| **Recovery Time** | Automatic restart |
| **Recommended** | Comprehensive error handling |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Replace unwrap() with ? | High | 1 day |
| Add error logging | High | 2 hours |
| Standardized error response | High | 4 hours |

---

### SF-002: Database Lock Contention

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Multiple writers blocked on Mutex |
| **Detection** | Slow writes, locked messages |
| **Impact** | Medium - Degraded performance |
| **Affected Features** | All write operations |
| **Current Mitigation** | None |
| **Recovery Time** | Automatic when lock releases |
| **Recommended** | Write queue, connection pooling |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Batch write operations | Medium | 1 day |
| Write queue with timeout | Low | 1 day |
| Monitor lock contention | Medium | 4 hours |

---

### SF-003: Memory Leaks

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Unbounded growth, memory not freed |
| **Detection** | Gradual memory increase, OOM |
| **Impact** | High - Eventually crashes |
| **Affected Features** | Long-running processes |
| **Current Mitigation** | None |
| **Recovery Time** | Process restart |
| **Recommended** | Memory profiling, leak detection |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Periodic memory profiling | Medium | 4 hours |
| Set memory limits | High | 1 hour |
| LRU cache for large data | Low | 1 day |

---

## Operational Failures

### OF-001: Misconfiguration

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Wrong URLs, missing credentials |
| **Detection** | Connection failures, auth errors |
| **Impact** | Variable - Feature or system failure |
| **Affected Features** | Configured features only |
| **Current Mitigation** | dotenvy with defaults |
| **Recovery Time** | Manual config fix |
| **Recommended** | Config validation on startup |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Startup config validation | High | 4 hours |
| Config documentation | Medium | 2 hours |
| Example config file | Medium | 1 hour |

---

### OF-002: Incorrect Data Entry

| Aspect | Details |
|--------|---------|
| **Failure Mode** | Manual data entry errors |
| **Detection** | Validation failures |
| **Impact** | Low - Individual record affected |
| **Affected Features** | Any feature using the data |
| **Current Mitigation** | None |
| **Recovery Time** | Manual correction |
| **Recommended** | Input validation, audit log |

| Mitigation | Priority | Effort |
|------------|----------|--------|
| Field validation | Medium | 4 hours |
| Change audit log | Low | 1 day |
| Admin approval workflow | Low | 2 days |

---

## Failure Severity Matrix

| Failure | Severity | Likelihood | Risk Score | Priority |
|---------|----------|------------|------------|----------|
| EDF-001: ArcGIS unavailable | Medium | Medium | 6 | High |
| EDF-002: Hydronet unavailable | High | Medium | 8 | Critical |
| EDF-003: EnergyZero unavailable | Medium | Low | 2 | Medium |
| DIF-001: DB corruption | Critical | Low | 3 | Critical |
| DIF-002: Invalid geometry | Low | Medium | 2 | Medium |
| DIF-003: Stale cache | Medium | High | 8 | High |
| IF-001: Disk exhaustion | Critical | Low | 3 | Critical |
| IF-002: Network partition | High | Low | 3 | High |
| IF-003: Memory exhaustion | Critical | Low | 3 | Critical |
| SF-001: Unhandled exceptions | High | Medium | 8 | Critical |
| SF-002: Lock contention | Medium | Low | 2 | Medium |
| SF-003: Memory leaks | High | Low | 2 | Medium |
| OF-001: Misconfiguration | High | Medium | 8 | High |
| OF-002: Data entry errors | Low | Medium | 2 | Low |

---

## Recovery Procedures

### Procedure 1: Database Corruption Recovery

```bash
# 1. Stop the service
systemctl stop peilbeheer-api

# 2. Backup corrupted file
cp data/peilbeheer.db data/peilbeheer.db.corrupted.$(date +%s)

# 3. Restore from latest backup
cp data/backups/peilbeheer.db.latest data/peilbeheer.db

# 4. Verify schema
duckdb data/peilbeheer.db "PRAGMA table_list;"

# 5. Restart service
systemctl start peilbeheer-api
```

---

### Procedure 2: External API Recovery

```bash
# 1. Verify API availability
curl -I https://arcgis.example.com/layer

# 2. Check recent error logs
journalctl -u peilbeheer-api --since "1 hour ago" | grep -i error

# 3. Manual sync trigger
curl -X POST http://localhost:8080/api/gemalen/sync

# 4. Verify data freshness
curl http://localhost:8080/api/gemalen | jq '.[0].fetched_at'
```

---

## Health Check Design

```rust
pub struct HealthStatus {
    pub healthy: bool,
    pub checks: Vec<HealthCheck>,
}

pub struct HealthCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub duration_ms: u64,
}

pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

// Required checks:
// 1. Database connectivity
// 2. Database write capability
// 3. ArcGIS availability
// 4. Hydronet availability
// 5. Disk space > 10%
// 6. Memory usage < 90%
```

---

*This document should be updated as new failure modes are identified.*
