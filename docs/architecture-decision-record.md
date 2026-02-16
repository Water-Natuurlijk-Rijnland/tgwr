# Architecture Decision Record

**Project:** Peilbeheer HHVR - Digital Twin Waterbeheer
**Version:** 0.1.0
**Date:** 2026-02-16

---

## Overview

This document records significant architectural decisions, their context, consequences, and current status. Each decision is tracked to understand the evolution of the system architecture.

---

## Decision Template

- **Status:** {Proposed | Accepted | Deprecated | Superseded}
- **Date:** YYYY-MM-DD
- **Context:** What is the issue that we're seeing that is motivating this decision or change?
- **Decision:** What is the change that we're proposing and/or doing?
- **Consequences:** What becomes easier or more difficult to do because of this change?
- **Alternatives Considered:** What other approaches did we consider and why were they rejected?

---

## ADR-001: Rust as Primary Language

**Status:** ✅ Accepted
**Date:** 2026-02-16
**Context:** Need for a systems programming language with memory safety, strong typing, and good async support for a digital twin water management system.

**Decision:** Use Rust (Edition 2024) as the primary programming language for all backend and simulation code.

**Consequences:**
- **Positive:**
  - Memory safety without GC overhead
  - Strong type system prevents entire classes of bugs
  - Excellent async support with Tokio
  - Zero-cost abstractions for performance-critical simulations
  - Single binary deployment simplifies operations
- **Negative:**
  - Steeper learning curve than Python/JavaScript
  - Longer compile times
  - Smaller talent pool for hiring

**Alternatives Considered:**
- **Python:** Rejected due to performance requirements for simulation
- **Go:** Rejected due to less expressive type system
- **Java/Kotlin:** Rejected due to JVM overhead and complexity

---

## ADR-002: Workspace Architecture with Four Crates

**Status:** ✅ Accepted
**Date:** 2026-02-16
**Context:** Need for clear separation of concerns between domain logic, simulation, API, and frontend.

**Decision:** Organize code as a Cargo workspace with four crates:
- `peilbeheer-core`: Domain models and shared types
- `peilbeheer-simulatie`: Simulation engine (water balance, PID, optimization)
- `peilbeheer-api`: REST API server and external integrations
- `peilbeheer-frontend`: Dioxus web application

**Consequences:**
- **Positive:**
  - Clear dependency boundaries
  - Core crate has zero dependencies on web/framework code
  - Can test simulation independently of API
  - Frontend can use core types directly
- **Negative:**
  - More complex build configuration
  - Longer build times due to multiple compilation units

**Alternatives Considered:**
- **Single crate:** Rejected due to lack of boundary enforcement
- **Microservices:** Rejected due to operational overhead

---

## ADR-003: DuckDB as Embedded Database

**Status:** ✅ Accepted
**Date:** 2026-02-16
**Context:** Need for persistent storage with analytical capabilities and spatial support for water management data.

**Decision:** Use DuckDB as an embedded analytical database with spatial extension.

**Consequences:**
- **Positive:**
  - No separate database service to manage
  - Single-file storage simplifies backups
  - Excellent analytical query performance (columnar)
  - Spatial extension for geo queries
  - Direct SQL from Rust without ORM overhead
- **Negative:**
  - Single-node only (no horizontal scaling)
  - Concurrent write performance limited
  - Smaller ecosystem than PostgreSQL

**Alternatives Considered:**
- **PostgreSQL + PostGIS:** Rejected due to deployment complexity
- **SQLite + Spatialite:** Rejected due to licensing and analytical performance
- **InfluxDB:** Rejected due to lack of spatial support

---

## ADR-004: Axum Web Framework

**Status:** ✅ Accepted
**Date:** 2026-02-16
**Context:** Need for a modern, async web framework with good ergonomics and Tower middleware support.

**Decision:** Use Axum 0.8 as the REST API framework.

**Consequences:**
- **Positive:**
  - Type-safe routing with extractors
  - Tower middleware ecosystem
  - Excellent Tokio integration
  - Minimal boilerplate
  - Good WebSocket support for future real-time features
- **Negative:**
  - Less mature than Actix-web
  - Smaller community

**Alternatives Considered:**
- **Actix-web:** Rejected due to more complex actor model
- **Rocket:** Rejected due to async limitations at time of decision

---

## ADR-005: Dioxus for Web Frontend

**Status:** ✅ Accepted
**Date:** 2026-02-16
**Context:** Need for a reactive web UI with strong type safety and Rust code sharing.

**Decision:** Use Dioxus for the web frontend, compiling to WebAssembly.

**Consequences:**
- **Positive:**
  - Share types between backend and frontend
  - Strong type safety across full stack
  - No separate JavaScript build pipeline
  - Compile-time optimizations
  - Small bundle size potential
- **Negative:**
  - Much smaller ecosystem than React
  - Less tooling and debugging support
  - Longer WASM compile times
  - Fewer third-party components

**Alternatives Considered:**
- **React + TypeScript:** Rejected due to duplication of type definitions
- **SvelteKit:** Rejected due to language boundary
- **Leptos:** Rejected due to less maturity than Dioxus

---

## ADR-006: GeoJSON for Spatial Data Exchange

**Status:** ✅ Accepted
**Date:** 2026-02-16
**Context:** Need to exchange spatial data (gemalen, assets, peilgebieden) between ArcGIS, database, and frontend.

**Decision:** Use GeoJSON as the canonical format for spatial data exchange throughout the system.

**Consequences:**
- **Positive:**
  - Standard format with wide tooling support
  - Human-readable for debugging
  - Direct support in DuckDB spatial extension
  - Leaflet can consume directly
- **Negative:**
  - Verbose compared to binary formats
  - No built-in topology support
  - Large file sizes for complex geometries

**Alternatives Considered:**
- **WKT/WKB:** Rejected due to poorer web support
- **Mapbox Vector Tiles:** Rejected due to complexity for current needs

---

## ADR-007: External API Integration Strategy

**Status:** ✅ Accepted
**Date:** 2026-02-16
**Context:** Need to integrate with three external systems (ArcGIS, Hydronet, EnergyZero) with different APIs and data formats.

**Decision:** Implement dedicated client modules for each external system with caching layer in DuckDB.

**Consequences:**
- **Positive:**
  - Clear separation of external concerns
  - Cached data improves resilience
  - Can evolve each client independently
- **Negative:**
  - Three separate implementations to maintain
  - Cache invalidation complexity
  - No unified error handling (yet)

**Future Consideration:** Implement a generic HTTP client wrapper with retry, circuit breaker, and metrics.

---

## ADR-008: Migration-Based Schema Management

**Status:** ✅ Accepted
**Date:** 2026-02-16
**Context:** Need for database schema evolution as the system grows.

**Decision:** Use numbered SQL migration files (001_*, 002_*, etc.) embedded at compile time.

**Consequences:**
- **Positive:**
  - Version-controlled schema changes
  - Reproducible database initialization
  - Can track which migrations applied
- **Negative:**
  - No automatic rollback mechanism
  - Schema changes require recompilation
  - No migration version tracking in database

**Alternatives Considered:**
- **Runtime migration loading:** Rejected due to deployment complexity
- **ORM migrations:** Rejected due to lack of suitable Rust ORM

---

## Superseded Decisions

*None yet*

---

## Proposed Decisions

| ID | Proposal | Status | Date Proposed |
|----|----------|--------|---------------|
| ADR-009 | Implement circuit breaker for external APIs | Proposed | TBD |
| ADR-010 | Add PostgreSQL migration path | Proposed | TBD |
| ADR-011 | Implement real-time updates via WebSocket | Proposed | TBD |

---

*This document should be updated with each significant architectural decision.*
