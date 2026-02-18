# Peilbeheer Simulatie - TODO

## ✅ Voltooid

- [x] Waterbalans berekening per tijdstap
- [x] Tijdreeks simulatie over periode
- [x] PID controller voor gemaal sturing
- [x] DP optimalisatie pompschema (enkel peilgebied)
- [x] Pomp vermogen berekening
- [x] Drooglegging berekening
- [x] Minimaal debiet vinden
- [x] Multi-peilgebied netwerksimulatie
- [x] Verbindingstypes: Gemaal, Overstort, Keerklep, OpenVerbinding
- [x] Coordinated control strategies

---

## 📋 Openstaand

### 1. Scenario Management
- [ ] Scenario's opslaan/laden (JSON/DB)
- [ ] Scenario vergelijking (before/after, what-if)
- [ ] Scenario templating
- [ ] Scenario versiebeheer
- [ ] Scenario export (rapport)

### 2. Geavanceerde Simulatie
- [ ] Boezem (shared buffer) modelling
- [ ] Kalender simulatie (seizoensvariatie)
- [ ] Historische data replay
- [ ] Monte Carlo simulatie (onzekerheid)
- [ ] Warmte/koude opslag in water

### 3. Validatie & Calibratie
- [ ] Resultaat validatie vs historische data
- [ ] Automatische kalibratie
- [ ] Onzekerheidsanalyse (confidence intervals)
- [ ] Model parameter tuning
- [ ] Cross-validation

### 4. Result Export & Rapportage
- [ ] CSV/JSON export van tijdstappen
- [ ] Grafiek generatie (time series plots)
- [ ] PDF rapportage templates
- [ ] KPI dashboard data
- [ ] Excel export met meerdere sheets

### 5. Real-time Control
- [ ] Live simulatie updates (websockets)
- [ ] Interruptible simulations
- [ ] State snapshot & restore
- [ ] Real-time parameter aanpassing
- [ ] Forecast integration (weer voorspelling)

### 6. Performance
- [ ] Parallelle simulatie van meerdere scenario's
- [ ] Incrementeel updaten van lopende simulaties
- [ ] Cached intermediate states
- [ ] SIMD optimalisatie voor bulk berekeningen

### 7. User Experience
- [ ] Visuele netwerk editor
- [ ] Interactive scenario builder
- [ ] Scenario wizard voor beginners
- [ ] Suggesties voor verbetering

---

## Prioriteit

| Taak | Prioriteit | Complexiteit |
|------|-----------|--------------|
| Scenario opslaan/laden | Hoog | Medium |
| CSV/JSON export | Hoog | Laag |
| Grafiek generatie | Medium | Medium |
| Boezem modelling | Medium | Hoog |
| Live simulatie updates | Laag | Hoog |
| Visuele netwerk editor | Laag | Hoog |
| Monte Carlo simulatie | Laag | Hoog |
