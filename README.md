# Peilbeheer HHVR - Digital Twin Waterbeheer

Een digitaal tweeling systeem voor waterbeheer in het Hoogheemraadschap van Rijnland en omstreken (HHVR), met real-time monitoring, simulatie en optimalisatiefunctionaliteiten.

## 🌊 Over dit Project

Dit project omvat:

1. **Digital Twin Waterbeheer** - Een volledig systeem voor real-time monitoring en optimalisatie
2. **TGWR Analyse** - Analyse van de spanning tussen lokale bezwaren en regionale TGWR ambities

## 🏗️ Digital Twin Systeem

### Architectuur

Het systeem is gebouwd met **Rust** en volgt de **AI-First SDLC v3** methodologie:

```
┌─────────────────────────────────────────────────────────┐
│                    Dioxus Frontend                       │
│  (Dashboard, Kaart, Simulatie, Real-time updates)       │
├─────────────────────────────────────────────────────────┤
│                    Axum REST API                         │
│  (Gemalen, Assets, Peilgebieden, Optimalisatie)         │
├─────────────────────────────────────────────────────────┤
│              Peilbeheer Simulatie Engine                 │
│  (Waterbalans, PID regeling, Energie-optimalisatie)     │
├─────────────────────────────────────────────────────────┤
│              Peilbeheer Core Domain                      │
│  (Gemaal, Asset, Peilgebied, Energie modellen)          │
├─────────────────────────────────────────────────────────┤
│                    DuckDB + Spatial                      │
│  (Embedded analytics, GeoJSON, Spatial queries)          │
└─────────────────────────────────────────────────────────┘
```

### Externe Integraties

| Systeem | Doel |
|---------|------|
| **ArcGIS** | Asset registratie (gemalen, stuwen, duikers) |
| **Hydronet** | Real-time debiet en status metingen |
| **EnergyZero** | Energieprijzen voor pomp optimalisatie |

### Functionaliteiten

- 📍 **Real-time monitoring** van 378+ gemalen
- 📊 **Waterbalans simulatie** met scenario analyse
- ⚡ **Energie-optimalisatie** op basis van uurprijzen
- 🗺️ **Interactieve kaart** met alle assets en peilgebieden
- 📈 **Trend analyse** met sliding window berekeningen

## 🚀 Quick Start

```bash
# Clone repository
git clone https://github.com/Water-Natuurlijk-Rijnland/tgwr.git
cd tgwr

# Backend draaien (dev)
cargo run --bin peilbeheer-api

# Frontend draaien (dev)
cd crates/peilbeheer-frontend
dx serve

# Productie build
cargo build --release
```

## 📁 Project Structuur

```
peilbeheer-hhvr/
├── crates/
│   ├── peilbeheer-core/      # Domeinmodellen
│   ├── peilbeheer-simulatie/  # Simulatie engine
│   ├── peilbeheer-api/        # REST API server
│   └── peilbeheer-frontend/   # Dioxus web app
├── migrations/                # Database schema
├── docs/                      # Architectuur documentatie
├── retrospectives/            # Project retrospectives
└── infographic.html           # TGWR analyse
```

## 📚 Documentatie

### Architectuur (AI-First SDLC v3)

- [Requirements Traceability Matrix](docs/requirements-traceability-matrix.md)
- [What-If Analysis](docs/what-if-analysis.md)
- [Architecture Decision Record](docs/architecture-decision-record.md)
- [System Invariants](docs/system-invariants.md)
- [Integration Design](docs/integration-design.md)
- [Failure Mode Analysis](docs/failure-mode-analysis.md)

### Feature Proposals

- [FP-001: Initial Water Management System](docs/feature-proposals/001-initial-water-management-system.md)
- [FP-002: SQL Schema Comment Stripping Fix](docs/feature-proposals/002-schema-comment-stripping-fix.md)

### Retrospectives

- [RETRO-001: Initial Project Setup](retrospectives/001-initial-project-setup.md)
- [RETRO-002: Schema Comment Stripping](retrospectives/002-schema-init-comment-stripping.md)

## 🛠️ Tech Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust 2024, Tokio, Axum |
| Frontend | Dioxus, WebAssembly |
| Database | DuckDB + Spatial Extension |
| Maps | Leaflet |
| Charts | Chart.js |
| CI/CD | GitHub Actions |

## 📊 TGWR Analyse

De [infographic.html](infographic.html) toont:

- **Lokale bezwaren** uit peilbesluiten van Rijnland
- **Regionale eisen** uit TGWR impactanalyse
- **Bestuurlijke vragen** uit Rekenkameronderzoek

### Kernparadox

Locale bezwaren tegen dynamisch peilbeheer zijn gebaseerd op **polderniveau-kosten**, terwijl TGWR vereist dat je kijkt naar **regionale baten**.

## 🤝 Bijdragen

Dit project volgt de AI-First SDLC v3 regels:

1. Maak eerst een [feature proposal](docs/feature-proposals/000-template.md)
2. Maak alle 6 architectuur documenten
3. Implementeer met zero technical debt
4. Werk de [retrospective](retrospectives/000-template.md) bij

## 📄 Licentie

MIT License - zie LICENSE bestand voor details.

---

_Software ontwikkeling met AI-First SDLC v3 - Waterschap Rijnland 2026_
