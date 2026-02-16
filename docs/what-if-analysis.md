# What-If Analysis

**Project:** Peilbeheer HHVR - Digital Twin Waterbeheer
**Version:** 0.1.0
**Date:** 2026-02-16
**Status:** Active

---

## Overview

This document explores alternative approaches, failure scenarios, and design trade-offs considered during system architecture. It serves as a record of decisions made and the reasoning behind them.

---

## Architectural Alternatives Considered

### 1. Database Technology

#### Alternative A: PostgreSQL with PostGIS
**Chosen:** DuckDB with Spatial Extension

| Factor | PostgreSQL + PostGIS | DuckDB + Spatial | Decision Rationale |
|--------|---------------------|------------------|-------------------|
| Deployment | Requires separate service | Embedded in binary | Simpler ops for MVP |
| Spatial support | Mature, industry standard | Emerging but adequate | PostGIS overkill for current use |
| Query performance | Good for transactional | Excellent for analytical | Workload is read-heavy analytics |
| Backup/restore | Complex | Single file copy | Operational simplicity |
| Scalability | Horizontal scaling possible | Single-node limited | Acceptable for current scale |

**Trade-off Acceptance:** Single-node limitation accepted in exchange for deployment simplicity. Can migrate to PostgreSQL later if needed.

#### Alternative B: SQLite with Spatialite
**Rejected** - Spatialite has licensing complexities and DuckDB offers better analytical query performance.

---

### 2. Frontend Framework

#### Alternative A: React/TypeScript
**Chosen:** Dioxus (Rust-based)

| Factor | React + TypeScript | Dioxus | Decision Rationale |
|--------|-------------------|--------|-------------------|
| Team expertise | Widely available | Niche | Learning curve offset by type safety |
| Build complexity | Node.js ecosystem | Cargo only | Unified build toolchain |
| Bundle size | Larger | Smaller | WASM optimization potential |
| Ecosystem maturity | Mature | Emerging | Sufficient for current needs |
| Type safety | With TypeScript | Native Rust | Stronger guarantees |

**Trade-off Acceptance:** Smaller ecosystem accepted. Can migrate to React if Dioxus becomes limiting.

---

### 3. Async Runtime

#### Alternative A: async-std
**Chosen:** Tokio

**Rationale:**
- Tokio is de facto standard in Rust ecosystem
- Better Axum integration
- Larger community and ecosystem
- More mature debugging tooling

---

### 4. API Architecture

#### Alternative A: GraphQL
**Chosen:** REST (Axum)

**Rationale:**
- Simpler to implement and debug
- Better caching with HTTP CDN
- Easier integration with external systems
- GraphQL overkill for current API surface

**Future Consideration:** GraphQL if self-service API requirements emerge.

---

## Failure Scenario Analysis

### Scenario 1: External API Outages

| System | Impact | Mitigation | Status |
|--------|--------|------------|--------|
| Hydronet unavailable | No real-time gemaal status | Cache last known status (7 days) | ✅ Implemented |
| ArcGIS unavailable | No asset/peilgebied updates | Serve from cache | ✅ Implemented |
| EnergyZero unavailable | No optimization | Fall back to time-based schedule | ⚠️ Partial |

**Recommendation:** Implement circuit breaker pattern for all external APIs.

---

### Scenario 2: Database Corruption

| Failure Mode | Impact | Mitigation |
|--------------|--------|------------|
| DuckDB file corruption | Complete data loss | Daily backups to object storage |
| Schema migration failure | Startup blocked | Rollback mechanism |
| Spatial extension failure | No geo queries | Fallback to bounding box queries |

**Status:** ❌ No backup mechanism implemented yet.

---

### Scenario 3: High Load Conditions

| Component | Current Limit | Failure Mode | Mitigation |
|-----------|--------------|--------------|------------|
| Axum server | ~10K concurrent connections | Request queuing | Request throttling |
| DuckDB | Single-threaded writes | Write lock contention | Batch writes |
| GeoJSON generation | O(n) for all features | Timeout on large datasets | Pagination/clipping |

**Status:** ⚠️ No load testing performed. No rate limiting implemented.

---

### Scenario 4: Data Quality Issues

| Issue Type | Impact | Detection | Recovery |
|------------|--------|-----------|----------|
| Missing coordinates | Can't map gemalen | Validation on insert | Exclude from map |
| Invalid peilgebieden | Spatial queries fail | ST_IsValid check | Log and skip |
| Negative debiet values | Incorrect calculations | Range validation | Clamp to 0 |

**Status:** ⚠️ Limited validation implemented.

---

## Scalability What-Ifs

### What If User Base Grows 10x?

| Component | Current | 10x Requirement | Upgrade Path |
|-----------|---------|-----------------|--------------|
| Database | Embedded DuckDB | ~1M queries/day | Migrate to PostgreSQL |
| API Server | Single instance | High availability | Kubernetes deployment |
| Frontend | Static files | CDN distribution | Cloudflare/CloudFront |
| External APIs | Current rate limits | Needs quota increase | Contract negotiation |

**Estimated Effort:** 2-3 weeks for migration architecture.

---

### What If Data Volume Grows 100x?

| Data Type | Current Volume | 100x Volume | Strategy |
|-----------|----------------|-------------|----------|
| Gemaal snapshots | ~400 | ~40K | Downsampling/ aggregation |
| Hourly averages | 7 days retention | 700 days | Tiered storage |
| GeoJSON | ~5MB | ~500MB | Vector tiles, simplification |

---

## Technology Migration Paths

### DuckDB → PostgreSQL
```sql
-- Migration would involve:
-- 1. Export DuckDB tables to CSV
-- 2. Create PostgreSQL schema with PostGIS
-- 3. Import and create indexes
-- 4. Update connection code (sqlx instead of duckdb)
```

**Estimated Effort:** 1-2 weeks for full migration.

### Dioxus → React
- API contracts remain unchanged
- Only frontend rewrite needed
- Can run in parallel during transition

**Estimated Effort:** 3-4 weeks for feature parity.

---

## Risk Register

| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| DuckDB file corruption | Low | Critical | Implement backups | TBD |
| ArcGIS rate limiting | Medium | High | Cache aggressively, request quota | TBD |
| WASM bundle size growth | Medium | Medium | Code splitting, lazy loading | TBD |
| External API changes | Medium | Medium | Version contracts, adapter pattern | TBD |
| Single point of failure (API) | High | Medium | Horizontal scaling readiness | TBD |

---

## Decision Log

| Date | Decision | Alternative Rejected | Rationale |
|------|----------|---------------------|-----------|
| 2026-02-16 | DuckDB over PostgreSQL | PostgreSQL | Simplicity for MVP |
| 2026-02-16 | Dioxus over React | React | Type safety, unified toolchain |
| 2026-02-16 | Axum over Actix-web | Actix-web | Ecosystem, middleware |
| 2026-02-16 | Embedded DB over client-server | Cloud DB | Deployment simplicity |

---

*This document should be updated when new architectural decisions are made.*
