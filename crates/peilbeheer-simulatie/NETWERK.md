# Multi-Peilgebied Netwerksimulatie

De `netwerk` module biedt ondersteuning voor het simuleren van waterstromen tussen meerdere verbonden peilgebieden.

## Concepten

### Netwerktopologie

Een netwerk bestaat uit:
- **Peilgebieden**: Elk peilgebied heeft eigen parameters (oppervlakte, streefpeil, marge, etc.)
- **Verbindingen**: Connecties tussen peilgebieden met verschillende types

### Verbindingstypes

1. **Gemaal**: Actieve watertransport met capaciteitsrestrictie
2. **Overstort**: Passieve stroming bij hoogwater boven drempel
3. **Keerklep**: Eenrichtingverkeer, alleen bij niveauverschil
4. **OpenVerbinding**: Tweerichtingsstroming op basis van niveauverschil

## Basis Voorbeeld

```rust
use peilbeheer_simulatie::netwerk::*;
use std::collections::HashMap;

// Maak topologie
let mut topologie = NetwerkTopologie::nieuw();

// Voeg peilgebieden toe
topologie.voeg_peilgebied_toe(PeilgebiedConfig {
    id: "polder_a".to_string(),
    naam: Some("Polder A".to_string()),
    oppervlakte: 100_000.0,
    streefpeil: -0.60,
    marge: 0.20,
    maaiveld_niveau: 0.0,
    max_uitstroom_debiet: 0.5,
    verdamping: 0.0,
    infiltratie: 0.0,
}).unwrap();

topologie.voeg_peilgebied_toe(PeilgebiedConfig {
    id: "polder_b".to_string(),
    naam: Some("Polder B".to_string()),
    oppervlakte: 80_000.0,
    streefpeil: -0.60,
    marge: 0.20,
    maaiveld_niveau: 0.0,
    max_uitstroom_debiet: 0.4,
    verdamping: 0.0,
    infiltratie: 0.0,
}).unwrap();

// Voeg verbinding toe
topologie.voeg_verbinding_toe(
    Verbinding::nieuw_gemaal(
        "verbinding_ab".to_string(),
        "polder_a".to_string(),
        "polder_b".to_string(),
        0.3,  // capaciteit (m³/s)
        2.0,  // opvoerhoogte (m)
    ).unwrap()
).unwrap();

// Start simulatie
let mut simulatie = NetwerkSimulatie::nieuw(topologie).unwrap();

// Optioneel: stel startwaterstand in
simulatie = simulatie.met_start_waterstand("polder_a", -0.40).unwrap();

// Simuleer één tijdstap
let mut regen = HashMap::new();
regen.insert("polder_a".to_string(), 5.0); // 5 mm/uur
regen.insert("polder_b".to_string(), 2.0); // 2 mm/uur

let strategy = SimpeleUitstroomStrategy;
let statussen = simulatie.simuleer_stap(&regen, &strategy).unwrap();

// Bekijk resultaten
for status in statussen {
    println!("{}: waterstand={:.2} m, inkomend={:.3} m³/s, uitgaand={:.3} m³/s",
        status.id, status.waterstand, status.inkomend_debiet, status.uitgaand_debiet);
}
```

## Geavanceerd Voorbeeld: Drie Peilgebieden

```rust
use peilbeheer_simulatie::netwerk::*;
use std::collections::HashMap;

let mut topologie = NetwerkTopologie::nieuw();

// Drie peilgebieden in cascade configuratie
for (id, naam, oppervlakte) in [
    ("hoog", "Hoogland", 150_000.0),
    ("midden", "Middengebied", 120_000.0),
    ("laag", "Laagland", 100_000.0),
] {
    topologie.voeg_peilgebied_toe(PeilgebiedConfig {
        id: id.to_string(),
        naam: Some(naam.to_string()),
        oppervlakte,
        streefpeil: -0.60,
        marge: 0.20,
        maaiveld_niveau: 0.0,
        max_uitstroom_debiet: 0.6,
        verdamping: 0.0,
        infiltratie: 0.0,
    }).unwrap();
}

// Gemaal van hoog naar midden
topologie.voeg_verbinding_toe(
    Verbinding::nieuw_gemaal(
        "gemaal_hm".to_string(),
        "hoog".to_string(),
        "midden".to_string(),
        0.4,
        1.5,
    ).unwrap()
).unwrap();

// Overstort van midden naar laag
topologie.voeg_verbinding_toe(
    Verbinding::nieuw_overstort(
        "overstort_ml".to_string(),
        "midden".to_string(),
        "laag".to_string(),
        0.5,
        -0.50, // drempelpeil
    ).unwrap()
).unwrap();

// Valideer en start
assert!(topologie.valideer().is_ok());
let mut simulatie = NetwerkSimulatie::nieuw(topologie).unwrap();

// Simuleer 24 uur
let mut regen_scenario = HashMap::new();
regen_scenario.insert("hoog".to_string(), vec![5.0; 24]); // Constante regen
regen_scenario.insert("midden".to_string(), vec![3.0; 24]);
regen_scenario.insert("laag".to_string(), vec![1.0; 24]);

let result = run_netwerksimulatie(
    &simulatie.topologie.clone(),
    &regen_scenario,
    24, // 24 uur
    &GebalanceerdeUitstroomStrategy::default(),
).unwrap();

println!("Simulatie voltooid: {} tijdstappen", result.tijdstappen.len());
```

## Strategieën voor Uitstroom

### SimpeleUitstroomStrategy

Pompt alleen als waterstand boven streefpeil:

```rust
let strategy = SimpeleUitstroomStrategy;
// Pompt max_debiet als ws > streefpeil, anders 0
```

### GebalanceerdeUitstroomStrategy

Houdt rekening met inkomende stroom voor balancering:

```rust
let strategy = GebalanceerdeUitstroomStrategy {
    balance_factor: 0.5,  // 0 = volledig reactief, 1 = volledig gebalanceerd
};
```

### Custom Strategy

Implementeer je eigen strategie:

```rust
struct AdaptiveStrategy {
    threshold: f64,
}

impl UitstroomStrategy for AdaptiveStrategy {
    fn bepaal_uitstroom(
        &self,
        _peilgebied_id: &str,
        waterstand: f64,
        config: &PeilgebiedConfig,
        _regen_intensiteit: f64,
        inkomend_debiet: f64,
    ) -> f64 {
        let afwijking = waterstand - config.streefpeil;
        if afwijking <= self.threshold {
            return 0.0;
        }

        // Schaal debiet met afwijking
        let basis = config.max_uitstroom_debiet * (afwijking / config.marge).min(1.0);

        // Compenseer voor inkomend debiet
        (basis + inkomend_debiet * 0.5).min(config.max_uitstroom_debiet)
    }
}
```

## Waterbalans per Peilgebied

De waterbalans wordt per tijdstap berekend als:

```
water_balans = regen + inkomend_verbindingen - uitgaand_verbindingen - uitstroom - verlies
waterstand_nieuw = waterstand_huidig + (water_balans / oppervlakte) * 60
```

Waarbij:
- `regen` in mm/uur (omgerekend naar m³/s)
- `inkomend_verbindingen` = som van alle debieten naar het peilgebied
- `uitgaand_verbindingen` = som van alle debieten uit het peilgebied
- `uitstroom` = debiet naar boezem/externe watergang
- `verlies` = verdamping + infiltratie

## Netwerkvalidatie

Het netwerk moet voldoen aan:

1. **Connectiviteit**: Alle peilgebieden moeten bereikbaar zijn (via verbindingen)
2. **Geen cycli**: Niet tegelijkertijd A→B en B→A voor actieve verbindingen
3. **Geldige parameters**: Oppervlakten > 0, debieten ≥ 0, etc.

```rust
topologie.valideer()?; // Retourneert Err bij problemen
```

## Integratie met Bestaande Simulatie

De netwerksimulatie integreert met de bestaande single-polder simulatie:

```rust
use peilbeheer_simulatie::waterbalans::{calculate_water_balance, mm_per_uur_to_m3_per_sec};

// In NetwerkSimulatie::simuleer_stap():
let balans = calculate_water_balance(
    regen_intensiteit,
    config.oppervlakte,
    huidige_waterstand,
    uitgaand + uitstroom_debiet,
    config.verdamping,
    config.infiltratie,
);
```

## Performance

- **Complexiteit**: O(V + E) per tijdstap, waarbij V = #peilgebieden, E = #verbindingen
- **Geheugen**: O(V) voor waterstanden, O(E) voor verbindingen
- **Simulatiesnelheid**: ~100.000 tijdstappen/sec op moderne hardware

## Foutafhandeling

Alle operaties retourneren `Result<T, NetwerkFout>`:

```rust
pub enum NetwerkFout {
    PeilgebiedNietGevonden { id: PeilgebiedId },
    VerbindingNietGevonden { id: VerbindingId },
    VerbindingBestaatAl { id: VerbindingId },
    CyclischeVerbinding { van: PeilgebiedId, naar: PeilgebiedId },
    OngeldigeCapaciteit { debiet: f64 },
    OngeldigeVerbinding { id: PeilgebiedId },
    NietVerbonden,
    ConstraintSchending { peilgebied: PeilgebiedId, waterstand: f64, min: f64, max: f64 },
}
```

## Toekomstige Uitbreidingen

Mogelijke extensies:

- **Multi-objective optimalisatie**: Balanceer energie, waterstand, en ecologische doelen
- **Predictive control**: Gebruik weersvoorspellingen voor proactieve regeling
- **Boezem buffering**: Model gedeelde buffergebieden
- **Tijdsafhankelijke parameters**: Seizoensgebonden streefpeilen, etc.
