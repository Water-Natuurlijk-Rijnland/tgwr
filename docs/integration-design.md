# Integration Design

**Project:** Peilbeheer HHVR - Digital Twin Waterbeheer
**Version:** 0.1.0
**Date:** 2026-02-16

---

## Overview

This document describes all external system integrations, their protocols, data flows, error handling strategies, and fallback mechanisms.

---

## Integration Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Peilbeheer HHVR                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │   ArcGIS    │    │  Hydronet   │    │ EnergyZero  │          │
│  │   Client    │    │   Client    │    │   Client    │          │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘          │
│         │                  │                  │                  │
│         ▼                  ▼                  ▼                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    DuckDB Cache                         │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## External Systems

### 1. ArcGIS REST API

**Purpose:** Asset registration (gemalen, stuwen, duikers, peilgebieden)

**Base URL:** Configurable via `ARCGIS_BASE_URL`

#### Endings

| Layer | Service | Layer ID | Type |
|-------|---------|----------|------|
| Gemalen | {service} | {id} | gemaal |
| Stuwen | {service} | {id} | stuw |
| Duikers | {service} | {id} | duiker |
| Peilgebieden | {service} | {id} | peilgebied |

#### Data Format

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [lon, lat]
      },
      "properties": {
        "CODE": "G001",
        "NAAM": "Gemaal A",
        "CAPACITEIT": 100,
        ...
      }
    }
  ]
}
```

#### Client Implementation
- **Module:** `arcgis_client.rs`
- **HTTP Client:** `reqwest`
- **Response Parsing:** `serde_json`

#### Caching Strategy
```rust
// Store in DuckDB asset_registratie table
// Fetched_at timestamp for freshness check
// Auto-sync on startup if cache empty
```

#### Error Handling
| Error Type | Current Handling | Recommended |
|------------|------------------|-------------|
| Connection failed | Log warning, serve from cache | ✅ Adequate |
| Invalid GeoJSON | Log error, skip | ✅ Adequate |
| Rate limit | Not handled | ❌ Add backoff |
| Timeout | 30s default | ⚠️ Consider reducing |

---

### 2. Hydronet API

**Purpose:** Real-time gemaal status and debiet measurements

**Base URL:** Configurable via `HYDRONET_BASE_URL`

#### Data Format
```json
{
  "data": [
    {
      "station_code": "G001",
      "timestamp": "2026-02-16T12:00:00Z",
      "value": 45.2,
      "status": "aan"
    }
  ]
}
```

#### Client Implementation
- **Module:** `hydronet_client.rs`
- **HTTP Client:** `reqwest`
- **Series Type:** `HydronetSeries` (multiple stations)

#### Caching Strategy
```rust
// Store snapshots in gemaal_status_snapshot
// Store hourly averages in gemaal_debiet_per_uur
// 7-day retention for hourly data
```

#### Error Handling
| Error Type | Current Handling | Recommended |
|------------|------------------|-------------|
| Station offline | Mark as Onbekend | ✅ Adequate |
| Partial data | Process available stations | ✅ Adequate |
| API down | Serve from cache | ✅ Adequate |
| Malformed response | Parse error, log | ⚠️ Need alerting |

---

### 3. EnergyZero API

**Purpose:** Hourly energy prices for pump scheduling optimization

**Base URL:** Configurable via `ENERGYZERO_BASE_URL`

#### Data Format
```json
{
  "data": [
    {
      "hour_start": "2026-02-16T12:00:00Z",
      "price_eur_per_mwh": 89.50
    }
  ]
}
```

#### Client Implementation
- **Module:** `energyzero_client.rs`
- **Usage:** Optimization engine for cost-based pumping

#### Caching Strategy
```rust
// Prices change hourly, cache with TTL
// No persistent storage currently
```

#### Error Handling
| Error Type | Current Handling | Recommended |
|------------|------------------|-------------|
| API down | No prices, optimization fails | ❌ Add fallback |
| Invalid data | Parse error | ⚠️ Need validation |
| Missing hours | Partial optimization | ⚠️ Handle gracefully |

---

## Internal Integration Points

### 1. API → Database

**Protocol:** Direct DuckDB connection via `duckdb` crate

**Connection Pooling:** Mutex-guarded single connection

**Operations:**
| Operation | Method | Concurrency |
|-----------|--------|-------------|
| Read snapshot | `get_snapshot()` | Concurrent reads OK |
| Write snapshot | `write_snapshot()` | Exclusive lock |
| Bulk upsert | `write_*_registraties()` | Exclusive lock |

**Future:** Consider read replicas for better concurrency

---

### 2. API → Simulation Engine

**Protocol:** Direct function call (same process)

**Interface:**
```rust
pub fn calculate_time_series(
    params: &SimulatieParams,
) -> Vec<SimulatieStap>
```

**No network overhead** - simulation runs in-process

---

### 3. Frontend → API

**Protocol:** HTTP/REST + WebSocket (future)

**CORS:** Currently allows all origins

**Endpoints:** See Requirements Traceability Matrix

**Data Format:** JSON for all responses

**Real-time:** Not implemented (future: WebSocket for live updates)

---

## Integration Patterns

### Current Patterns

| Pattern | Usage | Status |
|---------|-------|--------|
| Cache-Aside | ArcGIS, Hydronet data | ✅ Active |
| Lazy Loading | Auto-sync on empty cache | ✅ Active |
| Direct Coupling | Simulation → API | ✅ Active |
| Polling | Hydronet status updates | ⚠️ Inefficient |

### Recommended Patterns

| Pattern | Benefit | Priority |
|---------|---------|----------|
| Circuit Breaker | Prevent cascade failures | High |
| Retry with Backoff | Handle transient failures | High |
| Rate Limiting | Respect API quotas | Medium |
| Bulk Requests | Reduce round trips | Medium |
| Webhook | Push-based updates instead of polling | Low |
| Service Mesh | If microservices adopted | Future |

---

## Error Recovery Strategies

### Transient Errors (429, 503)

**Current:** No retry logic

**Recommended:**
```rust
pub async fn fetch_with_retry<T, F, Fut>(
    fetch_fn: F,
    max_retries: u32,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    loop {
        match fetch_fn().await {
            Ok(data) => return Ok(data),
            Err(e) if e.is_transient() && attempt < max_retries => {
                tokio::time::sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Permanent Errors (404, 400)

**Current:** Log and continue

**Status:** ✅ Appropriate

### Circuit Breaker Pattern

```rust
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure: AtomicInstant,
    state: AtomicState, // Open, Closed, Half-Open
}

impl CircuitBreaker {
    pub async fn call<F, R>(&self, f: F) -> Result<R>
    where F: FnOnce() -> Result<R>,
    {
        match self.state.load() {
            State::Open => Err(Error::CircuitOpen),
            State::Closed => f().map_err(|e| self.on_failure(e)),
            State::HalfOpen => f().map(|r| self.on_success(r))
                               .map_err(|e| self.on_failure(e)),
        }
    }
}
```

---

## Data Flow Diagrams

### Asset Registration Flow

```
┌─────────┐    HTTP GET    ┌──────────┐    Parse    ┌─────────┐
│ ArcGIS  │ ──────────────> │ API      │ ──────────> │ GeoJSON │
│ Server  │ <────────────── │ Server   │ <────────── │ Structs │
└─────────┘    200 OK       └────┬─────┘            └────┬────┘
                                    │                      │
                                    ▼                      ▼
                               ┌─────────┐    Upsert    ┌─────────┐
                               │ DuckDB  │ ───────────> │ Cache   │
                               │         │ <─────────── │ Updated │
                               └─────────┘    Rows     └─────────┘
```

### Real-time Status Update Flow

```
┌──────────┐    Poll     ┌──────────┐    Store    ┌─────────┐
│ Hydronet │ ──────────> │ API      │ ──────────> │ DuckDB  │
│ API      │ <────────── │ Server   │ <────────── │         │
└──────────┘    JSON      └────┬─────┘    Query    └─────────┘
                                 │
                                 ▼
                            ┌─────────┐    GET     ┌─────────┐
                            │ /status │ ──────────> │ Frontend│
                            │ endpoint│ <────────── │ Browser │
                            └─────────┘    JSON    └─────────┘
```

---

## Security Considerations

| Integration | Authentication | Encryption | Secrets |
|-------------|----------------|------------|---------|
| ArcGIS | API Key (if required) | HTTPS | ✅ .env |
| Hydronet | None (public) | HTTPS | N/A |
| EnergyZero | None (public) | HTTPS | N/A |

**Recommendations:**
1. Store API keys in environment variables
2. Add request signing for internal APIs
3. Consider mTLS for production deployments

---

## Monitoring & Observability

### Current Metrics
- ❌ No request/response logging
- ❌ No latency tracking
- ❌ No error rate monitoring

### Required Metrics

| Metric | Type | Threshold |
|--------|------|-----------|
| API request latency | Histogram | p95 < 1s |
| Error rate | Counter | < 5% |
| Cache hit rate | Gauge | > 80% |
| External API availability | Gauge | > 99% |

### Recommended Logging

```rust
tracing::info!(
    arcgis_layer = layer_type,
    feature_count = features.len(),
    duration_ms = start.elapsed().as_millis(),
    "ArcGIS sync completed"
);
```

---

## Testing Strategy

### Unit Tests
- Mock HTTP responses
- Test error paths
- Validate parsing

### Integration Tests
- ⚠️ Currently missing
- Should use test containers or mock servers

### Contract Tests
- ⚠️ Currently missing
- Should validate against API specs

---

*This document should be updated as new integrations are added.*
