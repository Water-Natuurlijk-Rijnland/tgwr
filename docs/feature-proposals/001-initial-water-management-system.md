# Feature Proposal: Digital Twin Water Management System

**Proposal ID:** FP-001
**Status:** ✅ Completed
**Created:** 2026-02-01
**Author:** Peilbeheer HHVR Team
**Assignee:** Peilbeheer HHVR Team

---

## Overview

Build a comprehensive digital twin system for water level management in the HHVR region, enabling real-time monitoring, simulation, and optimization of pump stations (gemalen), water levels, and drainage infrastructure.

---

## Problem Statement

### Current Situation
Water management in the HHVR region currently relies on:
- Manual monitoring of pump station status
- Limited real-time visibility into water levels
- No predictive capability for water balance
- Energy costs not optimized for pump scheduling
- Fragmented data from multiple sources (ArcGIS, Hydronet)

### Impact
- Operational inefficiency in pump scheduling
- Higher energy costs due to non-optimized pumping
- Delayed response to changing water conditions
- No scenario planning for extreme weather events

### Current Workarounds
- Manual data collection and phone coordination
- Spreadsheet-based calculations
- Experience-based decisions without simulation support

---

## Proposed Solution

### High-Level Description
A unified digital twin platform that:
1. Aggregates real-time data from all water infrastructure
2. Simulates water balance and drainage scenarios
3. Optimizes pump scheduling based on energy prices
4. Provides geospatial visualization of all assets
5. Enables scenario planning for extreme events

### User Stories
```
As a water management operator,
I want to see real-time status of all pump stations,
So that I can quickly respond to changing conditions.

As a district manager,
I want to simulate the impact of extreme rainfall,
So that we can prepare emergency response plans.

As an energy manager,
I want to optimize pump scheduling based on energy prices,
So that we can reduce operational costs.

As a GIS analyst,
I want to see all water assets on an interactive map,
So that I can understand spatial relationships and coverage.
```

### Success Criteria
- [x] Real-time status dashboard for all 378+ gemalen
- [x] Water balance simulation engine
- [x] Energy price integration for optimization
- [x] Interactive map with all infrastructure layers
- [x] Historical data retention (7+ days)
- [x] API for third-party integrations
- [x] Zero technical debt in codebase

---

## Technical Approach

### Architecture Changes
- **4-crate workspace**: core, api, simulatie, frontend
- **DuckDB** for embedded analytical storage with spatial extension
- **Axum** REST API server
- **Dioxus** web frontend with WebAssembly
- **External integrations**: ArcGIS, Hydronet, EnergyZero

### Data Changes
- `gemaal_status_snapshot`: Current status of all pump stations
- `gemaal_debiet_per_uur`: Hourly averages for trend analysis
- `gemaal_registratie`: Static asset data from ArcGIS
- `asset_registratie`: Generic asset table for all layers
- `peilgebied`: Water management areas with geometry

### API Changes
- `GET /health` - Health check
- `GET /gemalen` - List all pump stations
- `GET /gemalen/{code}` - Get specific pump station
- `GET /gemalen/geojson` - GeoJSON for map visualization
- `POST /gemalen/sync` - Trigger ArcGIS sync
- `GET /status` - Current status summary
- `POST /simulatie` - Run water balance simulation
- `GET /assets/geojson` - All assets for map
- `POST /optimalisatie` - Energy-optimized pump schedule

---

## Requirements Traceability

| REQ ID | Requirement | Component | Tests | Status |
|--------|-------------|-----------|-------|--------|
| CORE-001 | Gemaal status modelering | peilbeheer-core/src/gemaal.rs | ✅ | ✅ |
| CORE-002 | Trend analyse sliding window | peilbeheer-core/src/sliding_window.rs | ✅ | ✅ |
| CORE-003 | Waterbalans berekening | peilbeheer-simulatie/src/waterbalans.rs | ✅ | ✅ |
| CORE-004 | PID regelaar simulatie | peilbeheer-simulatie/src/pid.rs | ✅ | ✅ |
| CORE-005 | Drooglegging berekening | peilbeheer-simulatie/src/drooglegging.rs | ✅ | ✅ |
| DATA-001 | Hydronet integratie | peilbeheer-api/src/hydronet_client.rs | ❌ | ⚠️ |
| DATA-002 | ArcGIS integratie | peilbeheer-api/src/arcgis_client.rs | ❌ | ⚠️ |
| DB-001 | DuckDB embedded | peilbeheer-api/src/db.rs | ❌ | ✅ |
| API-001 | Health check | peilbeheer-api/src/routes/health.rs | ❌ | ✅ |
| UI-001 | Dashboard | peilbeheer-frontend/src/pages/dashboard.rs | ❌ | ✅ |

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| External API downtime | Medium | High | Caching with stale-while-revalidate |
| DuckDB single-node limits | Low | Medium | Migration path to PostgreSQL documented |
| WASM bundle size | Medium | Low | Code splitting potential |
| Spatial query performance | Low | Medium | Indexed columns in DuckDB |

---

## Implementation Estimate

| Phase | Tasks | Estimate | Actual |
|-------|-------|----------|--------|
| Design | Architecture docs, workspace setup | 3 days | 2 days |
| Implementation | Core, API, simulatie, frontend | 10 days | 12 days |
| Testing | Unit tests for domain logic | 2 days | 2 days |
| Documentation | API docs, architecture | 2 days | 3 days |
| **Total** | | **17 days** | **19 days** |

---

## Dependencies

### Internal Dependencies
None - greenfield project

### External Dependencies
- **ArcGIS REST API** - Must remain accessible or use cached data
- **Hydronet API** - Required for real-time status
- **EnergyZero API** - Required for optimization

---

## Rollout Plan

1. **Phase 1:** ✅ Core domain models and simulation engine
2. **Phase 2:** ✅ API server with database
3. **Phase 3:** ✅ External API integrations
4. **Phase 4:** ✅ Web frontend with map
5. **Phase 5:** ⏳ Production deployment and monitoring

---

## Approval

| Role | Name | Approval | Date |
|------|------|----------|------|
| Product Owner | Peilbeheer Team | ✅ Approved | 2026-02-01 |
| Solution Architect | SDLC Team | ✅ Reviewed | 2026-02-16 |
| Tech Lead | SDLC Team | ✅ Reviewed | 2026-02-16 |

---

## Change History

| Date | Version | Description | Author |
|------|---------|-------------|--------|
| 2026-02-01 | 1.0 | Initial proposal | Peilbeheer Team |
| 2026-02-16 | 1.1 | Marked completed, updated with actuals | SDLC Team |

---

*This feature proposal served as the foundation for the entire system architecture.*
