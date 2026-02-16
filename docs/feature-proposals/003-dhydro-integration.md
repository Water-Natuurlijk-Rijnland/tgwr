# Feature Proposal: DHYdro Platform Integration

**Proposal ID:** FP-003
**Status:** Proposed
**Created:** 2026-02-16
**Author:** Waterschap Rijnland Team
**Assignee:** TBD

---

## Overview

Integrate the DHYdro hydrological modeling platform with the Peilbeheer HHVR Digital Twin system, enabling advanced hydraulic modeling, scenario management, and bi-directional data exchange with Waterschap Rijnland's operational systems.

---

## Problem Statement

### Current Situation
The current Peilbeheer HHVR system provides:
- Real-time monitoring via Hydronet integration
- Water balance simulation with basic modeling
- Energy optimization for pump scheduling
- ArcGIS-based asset visualization

However, it lacks:
- Integration with the official DHYdro modeling platform
- Advanced hydraulic modeling capabilities
- Scenario persistence and comparison
- Enterprise authentication and authorization
- Real-time WebSocket updates

### Impact
- Operators cannot leverage DHYdro's advanced modeling capabilities
- No standardized scenario management across Rijnland
- Limited integration with Fews (Delft-FEWS forecasting system)
- Missing enterprise features for production deployment

### Current Workarounds
- Manual export/import of data between systems
- Separate DHYdro Cockpit access
- No unified operational view

---

## Proposed Solution

### High-Level Description
Implement full DHYdro platform integration including:

1. **DHYdro API Client** - Rust client library for DHYdro REST API
2. **Scenario Management** - Create, store, compare hydraulic modeling scenarios
3. **Fews Integration** - Time series data exchange with Delft-FEWS
4. **Authentication Layer** - OAuth 2.0 / JWT-based user management
5. **WebSocket Updates** - Real-time push notifications for status changes
6. **Enhanced Database** - PostgreSQL/PostGIS migration or hybrid approach

### User Stories
```
As a water management operator,
I want to run DHYdro hydraulic scenarios from the Peilbeheer interface,
So that I can compare different control strategies without switching systems.

As a data manager,
I want automatic synchronization between Fews and Peilbeheer,
So that forecast data is always up-to-date.

As a system administrator,
I want role-based access control,
So that operators can only modify approved scenarios.

As an operational manager,
I want real-time alerts on my mobile device,
So that I can respond quickly to critical situations.
```

### Success Criteria
- [ ] DHYdro API client with OAuth 2.0 authentication
- [ ] Scenario CRUD operations with persistence
- [ ] Fews time series bidirectional sync
- [ ] WebSocket real-time updates
- [ ] Role-based authentication (admin, operator, viewer)
- [ ] Alert rule engine with notifications
- [ ] PostgreSQL/PostGIS migration or hybrid solution
- [ ] Production deployment with Docker/Kubernetes

---

## Technical Approach

### Architecture Changes
```
┌─────────────────────────────────────────────────────────┐
│                    Dioxus Frontend                       │
│  (Dashboard, Scenario Manager, Real-time alerts)        │
├─────────────────────────────────────────────────────────┤
│                    Axum REST API + WebSocket             │
│  (Authenticated endpoints, Real-time broadcast)          │
├─────────────────────────────────────────────────────────┤
│              New: DHYdro Client Layer                   │
│  (DHYdro API, Fews client, Scenario manager)            │
├─────────────────────────────────────────────────────────┤
│              PostgreSQL/PostGIS (Primary)                │
│              DuckDB (Analytics) - Hybrid approach        │
├─────────────────────────────────────────────────────────┤
│   External: DHYdro API, Fews, ArcGIS, SCADA             │
└─────────────────────────────────────────────────────────┘
```

### New Modules

| Module | Path | Purpose |
|--------|------|---------|
| DHYdro Client | `peilbeheer-core/src/dhydro_client.rs` | DHYdro REST API integration |
| Fews Client | `peilbeheer-core/src/fews_client.rs` | Delft-FEWS PI/SQL integration |
| Auth Service | `peilbeheer-api/src/auth.rs` | JWT authentication, RBAC |
| WebSocket Handler | `peilbeheer-api/src/websocket.rs` | Real-time broadcast |
| Scenario Service | `peilbeheer-api/src/scenario.rs` | Scenario CRUD operations |
| Alert Engine | `peilbeheer-core/src/alerting.rs` | Alert rule evaluation |

### Database Schema Changes

```sql
-- Users and authentication
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username VARCHAR UNIQUE NOT NULL,
    email VARCHAR UNIQUE NOT NULL,
    password_hash VARCHAR NOT NULL,
    role VARCHAR NOT NULL, -- admin, operator, viewer
    created_at TIMESTAMP DEFAULT NOW()
);

-- Scenarios
CREATE TABLE scenarios (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    description TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP DEFAULT NOW(),
    parameters JSONB NOT NULL,
    base_scenario_id UUID REFERENCES scenarios(id)
);

-- Scenario results
CREATE TABLE scenario_results (
    id UUID PRIMARY KEY,
    scenario_id UUID REFERENCES scenarios(id),
    executed_at TIMESTAMP DEFAULT NOW,
    status VARCHAR NOT NULL, -- running, completed, failed
    results JSONB,
    error_message TEXT
);

-- Alert rules
CREATE TABLE alert_rules (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    condition_type VARCHAR NOT NULL, -- threshold, rate_of_change, pattern
    parameters JSONB NOT NULL,
    notification_channels JSONB NOT NULL, -- email, webhook, sms
    created_by UUID REFERENCES users(id)
);

-- Alert history
CREATE TABLE alert_history (
    id UUID PRIMARY KEY,
    rule_id UUID REFERENCES alert_rules(id),
    triggered_at TIMESTAMP DEFAULT NOW(),
    severity VARCHAR NOT NULL,
    message TEXT,
    acknowledged BOOLEAN DEFAULT FALSE
);
```

### API Changes

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/api/auth/login` | POST | Public | JWT token generation |
| `/api/auth/refresh` | POST | Public | Token refresh |
| `/api/scenarios` | GET | Authenticated | List scenarios |
| `/api/scenarios` | POST | Admin+ | Create scenario |
| `/api/scenarios/{id}` | GET | Authenticated | Get scenario |
| `/api/scenarios/{id}` | PUT | Admin+ | Update scenario |
| `/api/scenarios/{id}/run` | POST | Operator+ | Execute scenario |
| `/api/scenarios/{id}/results` | GET | Authenticated | Get results |
| `/api/fews/timeseries` | GET | Authenticated | Fetch from Fews |
| `/api/fews/sync` | POST | Admin+ | Trigger sync |
| `/api/alerts/rules` | CRUD | Admin+ | Alert rule management |
| `/api/alerts/acknowledge` | POST | Operator+ | Acknowledge alert |
| `/api/ws` | WebSocket | Authenticated | Real-time updates |

---

## Requirements Traceability

| REQ ID | Requirement | Component | Tests | Status |
|--------|-------------|-----------|-------|--------|
| DHY-001 | DHYdro API client | dhdro_client.rs | ❌ | Pending |
| DHY-002 | OAuth 2.0 authentication | auth.rs | ❌ | Pending |
| DHY-003 | Scenario persistence | scenario.rs | ❌ | Pending |
| DHY-004 | Fews PI/SQL client | fews_client.rs | ❌ | Pending |
| DHY-005 | WebSocket broadcast | websocket.rs | ❌ | Pending |
| DHY-006 | Alert rule engine | alerting.rs | ❌ | Pending |
| DHY-007 | PostgreSQL migration | db.rs + migrations | ❌ | Pending |
| DHY-008 | Role-based access control | auth.rs | ❌ | Pending |

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| DHYdro API changes | Medium | High | Version contracts, adapter pattern |
| PostgreSQL migration complexity | High | High | Hybrid approach, gradual migration |
| Fews integration challenges | Medium | Medium | PI/SQL library, fallback to CSV |
| WebSocket scalability | Low | Medium | Connection pooling, load testing |
| Authentication security flaws | Low | Critical | Security audit, penetration testing |

---

## Implementation Estimate

| Phase | Tasks | Estimate |
|-------|-------|----------|
| Phase 1: DHYdro Client | API client, OAuth, basic endpoints | 5 days |
| Phase 2: Scenario Management | Persistence, CRUD, execution | 5 days |
| Phase 3: Fews Integration | PI/SQL client, sync logic | 4 days |
| Phase 4: Authentication | JWT, RBAC, user management | 3 days |
| Phase 5: WebSocket | Real-time updates, subscriptions | 3 days |
| Phase 6: Database Migration | PostgreSQL schema, data migration | 5 days |
| Phase 7: Alerting | Rule engine, notifications | 4 days |
| Phase 8: Production | Docker, K8s, monitoring, docs | 5 days |
| **Total** | | **34 days (~7 weeks)** |

---

## Dependencies

### Internal Dependencies
- FP-002: Schema comment stripping (merged)
- Existing ArcGIS and Hydronet clients

### External Dependencies
- **DHYdro API access** - API credentials and documentation
- **Fews system access** - PI/SQL connection details
- **OAuth provider** - Identity provider configuration
- **PostgreSQL infrastructure** - Database server provisioning

---

## Rollout Plan

1. **Phase 1:** DHYdro client development with sandbox access
2. **Phase 2:** Scenario management with DuckDB backend
3. **Phase 3:** Fews integration in development environment
4. **Phase 4:** Authentication layer with internal users
5. **Phase 5:** WebSocket support for beta users
6. **Phase 6:** PostgreSQL migration (parallel testing)
7. **Phase 7:** Alert rule engine for operators
8. **Phase 8:** Production deployment with monitoring

---

## Approval

| Role | Name | Approval | Date |
|------|------|----------|------|
| Product Owner | | ⬜ Approved | |
| Solution Architect | | ⬜ Reviewed | |
| Tech Lead | | ⬜ Reviewed | |
| DHYdro Liaison | | ⬜ Reviewed | |

---

## Change History

| Date | Version | Description | Author |
|------|---------|-------------|--------|
| 2026-02-16 | 1.0 | Initial proposal based on DHYdro implementation plan | SDLC Team |

---

## References

- DHYdro Implementation Plan: `docs/implementatieplan_dhydro_rijnland.pdf`
- Current System Architecture: `docs/architecture-decision-record.md`
- Integration Design: `docs/integration-design.md`
- Gap Analysis: Generated by AI agent on 2026-02-16

---

*This proposal represents a significant expansion of the Peilbeheer HHVR system to achieve full DHYdro platform alignment.*
