# Requirements Traceability Matrix

**Project:** Peilbeheer HHVR - Digital Twin Waterbeheer
**Version:** 0.1.0
**Date:** 2026-02-16
**Status:** Active

---

## Overview

This document traces all system requirements from their origin through implementation, testing, and deployment. Each requirement is uniquely identified and linked to its corresponding architecture components, code modules, and test cases.

---

## Requirement Categories

1. **Core Domain** - Water management domain models
2. **Data Acquisition** - External system integrations
3. **Simulation Engine** - Water balance and optimization
4. **API Layer** - REST API and data serving
5. **User Interface** - Web frontend
6. **Data Persistence** - Database operations

---

## Requirements Traceability Table

| REQ ID | Requirement | Source | Component | Module/File | Tests | Status |
|--------|-------------|--------|------------|-------------|-------|--------|
| **CORE-001** | Gemaal status modelering | Domain | peilbeheer-core | `src/gemaal.rs` | `test_gemaal_status_from_str` | ✅ Implemented |
| **CORE-002** | Trend analyse met sliding window | Domain | peilbeheer-core | `src/sliding_window.rs` | Multiple trend tests | ✅ Implemented |
| **CORE-003** | Waterbalans berekening | Domain | peilbeheer-simulatie | `src/waterbalans.rs` | 6 test functions | ✅ Implemented |
| **CORE-004** | PID regelaar simulatie | Domain | peilbeheer-simulatie | `src/pid.rs` | 5 test functions | ✅ Implemented |
| **CORE-005** | Drooglegging berekening | Domain | peilbeheer-simulatie | `src/drooglegging.rs` | 3 test functions | ✅ Implemented |
| **CORE-006** | Asset registratie model | Domain | peilbeheer-core | `src/asset.rs` | - | ✅ Implemented |
| **CORE-007** | Peilgebied model met geometrie | Domain | peilbeheer-core | `src/peilgebied.rs` | - | ✅ Implemented |
| **CORE-008** | Energieprijs optimalisatie | Domain | peilbeheer-core | `src/energie.rs` | 8 test functions | ✅ Implemented |
| **DATA-001** | Hydronet tijdreeks data ophalen | External API | peilbeheer-api | `src/hydronet_client.rs` | - | ✅ Implemented |
| **DATA-002** | ArcGIS GeoJSON integratie | External API | peilbeheer-api | `src/arcgis_client.rs` | - | ✅ Implemented |
| **DATA-003** | EnergyZero prijs data | External API | peilbeheer-api | `src/energyzero_client.rs` | - | ✅ Implemented |
| **DB-001** | DuckDB embedded database | Infrastructure | peilbeheer-api | `src/db.rs` | - | ✅ Implemented |
| **DB-002** | Spatial extensie voor geometrie | Infrastructure | peilbeheer-api | `src/db.rs` | - | ✅ Implemented |
| **DB-003** | Gemaal status snapshots | Data Model | migrations | `001_initial_schema.sql` | - | ✅ Implemented |
| **DB-004** | Uurgemiddelden debiet | Data Model | migrations | `001_initial_schema.sql` | - | ✅ Implemented |
| **DB-005** | Asset registratie caching | Data Model | migrations | `002_asset_registratie.sql` | - | ✅ Implemented |
| **DB-006** | Peilgebieden met geometrie | Data Model | migrations | `003_peilgebieden.sql` | - | ✅ Implemented |
| **API-001** | Health check endpoint | REST API | peilbeheer-api | `src/routes/health.rs` | - | ✅ Implemented |
| **API-002** | Gemalen lijst endpoint | REST API | peilbeheer-api | `src/routes/gemalen.rs` | - | ✅ Implemented |
| **API-003** | Gemaal detail endpoint | REST API | peilbeheer-api | `src/routes/gemalen.rs` | - | ✅ Implemented |
| **API-004** | Status summary endpoint | REST API | peilbeheer-api | `src/routes/status.rs` | - | ✅ Implemented |
| **API-005** | Simulatie endpoint | REST API | peilbeheer-api | `src/routes/simulatie.rs` | - | ✅ Implemented |
| **API-006** | Assets GeoJSON endpoint | REST API | peilbeheer-api | `src/routes/assets.rs` | - | ✅ Implemented |
| **API-007** | Peilgebieden GeoJSON endpoint | REST API | peilbeheer-api | `src/routes/peilgebieden.rs` | - | ✅ Implemented |
| **API-008** | Optimalisatie endpoint | REST API | peilbeheer-api | `src/routes/optimalisatie.rs` | - | ✅ Implemented |
| **UI-001** | Dashboard pagina | Frontend | peilbeheer-frontend | `src/pages/dashboard.rs` | - | ✅ Implemented |
| **UI-002** | Gemalen overzicht | Frontend | peilbeheer-frontend | `src/pages/gemalen.rs` | - | ✅ Implemented |
| **UI-003** | Gemaal detail pagina | Frontend | peilbeheer-frontend | `src/pages/gemaal_detail.rs` | - | ✅ Implemented |
| **UI-004** | Simulatie pagina | Frontend | peilbeheer-frontend | `src/pages/simulatie.rs` | - | ✅ Implemented |
| **UI-005** | Leaflet kaart integratie | Frontend | peilbeheer-frontend | `src/components/map.rs` | - | ✅ Implemented |
| **UI-006** | Status badge component | Frontend | peilbeheer-frontend | `src/components/status_badge.rs` | - | ✅ Implemented |
| **UI-007** | Navigatie balk | Frontend | peilbeheer-frontend | `src/components/navbar.rs` | - | ✅ Implemented |

---

## Verification Matrix

### Requirements to Tests Coverage

| Requirement | Unit Tests | Integration Tests | E2E Tests | Coverage % |
|-------------|------------|-------------------|-----------|------------|
| CORE-001 to CORE-008 | 28 | 0 | 0 | 35% |
| DATA-001 to DATA-003 | 0 | 0 | 0 | 0% |
| DB-001 to DB-006 | 0 | 0 | 0 | 0% |
| API-001 to API-008 | 0 | 0 | 0 | 0% |
| UI-001 to UI-007 | 0 | 0 | 0 | 0% |

**Overall Test Coverage: ~8%** (limited to core domain logic)

---

## Change History

| Date | Version | Description | Author |
|------|---------|-------------|--------|
| 2026-02-16 | 1.0 | Initial RTM based on existing codebase | AI-First SDLC |

---

## Notes

1. **Gap Analysis**: Integration and E2E tests are missing. Test coverage needs significant expansion.

2. **External Dependencies**: ArcGIS, Hydronet, and EnergyZero integrations need circuit breaker patterns and retry logic.

3. **Data Freshness**: Requirements for auto-sync on startup are implemented but lack monitoring/alerting.

4. **Spatial Operations**: PostGIS-like operations via DuckDB spatial extension need validation for production use.

---

*This document is living and should be updated with every requirement change.*
