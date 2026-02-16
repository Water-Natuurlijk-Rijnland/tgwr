# System Invariants

**Project:** Peilbeheer HHVR - Digital Twin Waterbeheer
**Version:** 0.1.0
**Date:** 2026-02-16

---

## Overview

System invariants are properties that must always hold true for the system to function correctly. This document defines all invariants, their enforcement mechanisms, and validation approaches.

---

## Invariant Categories

1. **Data Invariants** - Data integrity and consistency
2. **Domain Invariants** - Business logic constraints
3. **System Invariants** - Operational guarantees
4. **API Invariants** - Interface contracts

---

## Data Invariants

### DI-001: Gemaal Code Uniqueness

**Invariant:** Each gemaal (pump station) has a unique code that never changes.

**Scope:** `gemaal_registratie.code`, `gemaal_status_snapshot.gemaal_code`

**Enforcement:**
```sql
-- Primary key ensures uniqueness
CREATE TABLE gemaal_registratie (
    code VARCHAR PRIMARY KEY,  -- Enforcement point
    ...
);
```

**Validation:**
- Database constraint prevents duplicates
- ArcGIS sync uses `ON CONFLICT` to update existing codes

**Violation Impact:** Medium - Could cause status updates to affect wrong gemaal

---

### DI-002: Debiet Non-Negative

**Invariant:** Flow rate (debiet) is always >= 0 m³/s.

**Scope:** `gemaal_status_snapshot.debiet`, `gemaal_debiet_per_uur.avg_debiet`

**Enforcement:**
```rust
// Currently enforced at domain level
pub struct GemaalSnapshot {
    pub debiet: f64,  // Should have validation
}
```

**Current Status:** ⚠️ **No validation enforced** - negative values possible

**Required Fix:**
```rust
impl GemaalSnapshot {
    pub fn new(debiet: f64) -> Self {
        assert!(debiet >= 0.0, "debiet must be non-negative");
        // ...
    }
}
```

---

### DI-003: Timestamp Ordering

**Invariant:** `generated_at` timestamp is always >= `last_update` timestamp.

**Scope:** `gemaal_status_snapshot`

**Enforcement:** None currently

**Current Status:** ⚠️ **Not enforced**

**Required Fix:** Add validation in Database::write_snapshot()

---

### DI-004: Asset Unique Key

**Invariant:** Each asset is uniquely identified by `(layer_type, code)`.

**Scope:** `asset_registratie`

**Enforcement:**
```sql
CREATE TABLE asset_registratie (
    layer_type VARCHAR NOT NULL,
    code VARCHAR NOT NULL,
    PRIMARY KEY (layer_type, code)  -- Composite key
);
```

**Status:** ✅ Enforced by database

---

## Domain Invariants

### DM-001: Gemaal Status Enumeration

**Invariant:** Gemaal status can only be one of: `Aan`, `Uit`, `Onbekend`, `Error`.

**Scope:** `GemaalStatus` enum

**Enforcement:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemaalStatus {
    Aan,
    Uit,
    Onbekend,
    Error,
}
```

**Status:** ✅ Enforced by Rust type system

---

### DM-002: Water Balance Conservation

**Invariant:** In a closed system, water in = water out + storage change.

**Scope:** `waterbalans::calculate_water_balance()`

**Enforcement:**
```rust
pub fn calculate_water_balance(
    influx: f64,
    outflow: f64,
    storage: f64,
) -> f64 {
    influx - outflow - storage  // Should approach 0 for balanced system
}
```

**Validation:** Unit tests verify calculation correctness

**Status:** ✅ Enforced with tests

---

### DM-003: Trend Calculation Window Size

**Invariant:** Trend analysis requires minimum data points equal to window size.

**Scope:** `sliding_window::SlidingWindowProcessor`

**Enforcement:**
```rust
impl SlidingWindowProcessor {
    pub fn calculate_trend(&self) -> Option<TrendInfo> {
        if self.data.len() < MIN_POINTS {
            return None;  // Cannot calculate trend
        }
        // ...
    }
}
```

**Status:** ✅ Returns `Option` to handle insufficient data

---

### DM-004: Peilgebied Spatial Validity

**Invariant:** Every peilgebied must have a valid geometry (non-empty, valid polygon).

**Scope:** `peilgebied.geometry`

**Enforcement:** DuckDB spatial extension `ST_IsValid()`

**Current Status:** ⚠️ **Not validated on insert**

**Required Fix:**
```rust
// Validate geometry before insert
conn.execute(
    "INSERT INTO peilgebied ... WHERE ST_IsValid(?)",
    [geometry]
)
```

---

## System Invariants

### SI-001: Single Database Writer

**Invariant:** Only one database connection writes at a time (enforced by Mutex).

**Scope:** `db::Database`

**Enforcement:**
```rust
pub struct Database {
    conn: Mutex<Connection>,  // Single writer guarantee
}
```

**Status:** ✅ Enforced by Rust Mutex

**Limitation:** Becomes bottleneck under high write load

---

### SI-002: Auto-Sync Only on Empty Cache

**Invariant:** Auto-sync from external APIs only runs when cache is empty (registratie_count == 0).

**Scope:** `main.rs` startup logic

**Enforcement:**
```rust
let registratie_count = db.get_registratie_count().unwrap_or(0);
if registratie_count == 0 {
    // Only sync if empty
}
```

**Status:** ✅ Enforced

**Issue:** No manual sync endpoint override for refresh

---

### SI-003: Server Binds Successfully

**Invariant:** Server only starts if database initialization succeeds.

**Scope:** `main.rs`

**Enforcement:**
```rust
db.initialize_schema()?;  // Fails fast if schema fails
axum::serve(listener, app).await?;
```

**Status:** ✅ Enforced with `?` operator

---

### SI-004: CORS All Origins

**Invariant:** API accepts requests from any origin (development mode invariant).

**Scope:** CORS Layer

**Enforcement:**
```rust
CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any),
```

**Status:** ⚠️ **Too permissive for production**

**Required Fix:** Restrict to specific domains in production

---

## API Invariants

### API-001: Response Format Consistency

**Invariant:** All successful API responses return JSON with consistent structure.

**Scope:** All route handlers

**Enforcement:** Axum `Json<>` extractor

**Status:** ✅ Enforced at type level

---

### API-002: Error Response Structure

**Invariant:** All errors return structured error response with message.

**Scope:** Error handling

**Current Status:** ⚠️ **Inconsistent** - some return 500, some have custom errors

**Required Fix:** Standardize error response type

---

### API-003: GeoJSON Always Valid

**Invariant:** All `/geojson` endpoints return valid GeoJSON FeatureCollection.

**Scope:** GeoJSON endpoints

**Validation:** Can be validated with `geojson-validator`

**Status:** ⚠️ **No runtime validation**

---

## Invariant Violation Handling

### Current Approach
- Database constraints: Log warning, continue
- Parse errors: Return default/empty values
- Network errors: Log, serve from cache

### Required Approach
```rust
pub enum InvariantViolation {
    Critical(&'static str),  // Shutdown system
    Warning(&'static str),   // Log and alert
    Recoverable(Box<dyn Fn() -> Result<()>>)  // Attempt recovery
}
```

---

## Validation Checklist

| Invariant | Enforcement | Tests | Monitoring | Status |
|-----------|-------------|-------|------------|--------|
| DI-001: Gemaal uniqueness | DB constraint | ❌ | ❌ | ⚠️ Partial |
| DI-002: Debiet non-negative | None | ❌ | ❌ | ❌ Missing |
| DI-003: Timestamp ordering | None | ❌ | ❌ | ❌ Missing |
| DI-004: Asset unique key | DB constraint | ❌ | ❌ | ⚠️ Partial |
| DM-001: Status enum | Type system | ✅ | N/A | ✅ Complete |
| DM-002: Water balance | Tests | ✅ | ❌ | ✅ Complete |
| DM-003: Trend window | Option type | ✅ | ❌ | ✅ Complete |
| DM-004: Geometry validity | None | ❌ | ❌ | ❌ Missing |
| SI-001: Single writer | Mutex | ❌ | ❌ | ⚠️ Partial |
| SI-002: Auto-sync condition | If check | ❌ | ❌ | ✅ Complete |
| SI-003: DB init success | ? operator | ❌ | ❌ | ✅ Complete |
| SI-004: CORS restriction | None | ❌ | ❌ | ❌ Missing |

---

## Future Invariants to Add

1. **Debiet Rate Limit:** Max 1000 m³/s (physical impossibility check)
2. **Coordinate Bounds:** Lat/Lon must be within Netherlands bounds
3. **Timestamp Freshness:** `last_update` cannot be > 24 hours old
4. **Peilgebied Coverage:** All gemalen must belong to a peilgebied

---

*This document should be updated as new invariants are identified.*
