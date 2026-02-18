// Voorbeeld van grafiek generatie.
//
// Dit voorbeeld demonstreert:
// - Waterstand grafiek generatie
// - Regenintensiteit grafiek generatie
// - Pompcyclus grafiek generatie
// - Export naar PNG en SVG

use peilbeheer_simulatie::netwerk::*;
use peilbeheer_simulatie::visualisatie::*;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Grafiek Generatie Voorbeeld ===\n");

    // Stap 1: Voer een scenario uit
    println!("1. Scenario uitvoeren...");
    let resultaat = voer_scenario()?;

    println!(
        "   Simulatie voltooid: {} tijdstappen\n",
        resultaat.tijdstappen.len()
    );

    // Stap 2: Genereer waterstand grafiek
    println!("2. Waterstand grafiek genereren...");
    let waterstand_opties = GrafiekOpties {
        titel: Some("Waterstand Verloop - Regen Scenario".to_string()),
        x_label: Some("Tijd (minuten)".to_string()),
        y_label: Some("Waterstand (m NAP)".to_string()),
        ..Default::default()
    };

    let waterstand_grafiek = WaterstandGrafiek::met_opties(waterstand_opties);
    waterstand_grafiek.naar_png(&resultaat, "waterstand.png")?;
    println!("   Opgeslagen: waterstand.png");

    // Stap 3: Genereer SVG versie
    waterstand_grafiek.naar_svg(&resultaat, "waterstand.svg")?;
    println!("   Opgeslagen: waterstand.svg");

    // Stap 4: Genereer regen grafiek
    println!("\n3. Regenintensiteit grafiek genereren...");
    let regen_opties = GrafiekOpties {
        titel: Some("Regenintensiteit".to_string()),
        ..Default::default()
    };

    let regen_grafiek = RegenGrafiek::met_opties(regen_opties);
    regen_grafiek.naar_png(&resultaat, "regen.png")?;
    println!("   Opgeslagen: regen.png");

    // Stap 5: Genereer pomp grafiek
    println!("\n4. Pompcyclus grafiek genereren...");
    let pomp_opties = GrafiekOpties {
        titel: Some("Pompcycli per Peilgebied".to_string()),
        ..Default::default()
    };

    let pomp_grafiek = PompGrafiek::met_opties(pomp_opties);
    pomp_grafiek.naar_png(&resultaat, "pompcycli.png")?;
    println!("   Opgeslagen: pompcycli.png");

    // Stap 6: Genereer alle grafieken in één keer
    println!("\n5. Alle grafieken genereren in map...");
    let gegenereerd = genereer_alle_grafieken(&resultaat, ".")?;

    for (naam, pad) in &gegenereerd {
        println!("   - {}: {}", naam, pad);
    }

    println!("\n=== Generatie voltooid ===");
    println!("\nGegenereerde bestanden:");
    println!("  - waterstand.png");
    println!("  - waterstand.svg");
    println!("  - regen.png");
    println!("  - pompcycli.png");

    Ok(())
}

fn voer_scenario() -> Result<NetwerkSimulatieResultaat, Box<dyn std::error::Error>> {
    // Maak topologie
    let mut topologie = NetwerkTopologie::nieuw();

    let peilgebieden = vec![
        ("polder_noord", "Noordelijke Polder", 200_000.0, -0.50),
        ("polder_midden", "Middenpolder", 150_000.0, -0.55),
        ("polder_zuid", "Zuidelijke Polder", 120_000.0, -0.60),
    ];

    for (id, naam, oppervlakte, streefpeil) in peilgebieden {
        topologie.voeg_peilgebied_toe(PeilgebiedConfig {
            id: id.to_string(),
            naam: Some(naam.to_string()),
            oppervlakte,
            streefpeil,
            marge: 0.20,
            maaiveld_niveau: 0.0,
            max_uitstroom_debiet: 0.8,
            verdamping: 0.1,
            infiltratie: 0.05,
        })?;
    }

    // Gemaalverbindingen
    topologie.voeg_verbinding_toe(Verbinding::nieuw_gemaal(
        "gemaal_nm".to_string(),
        "polder_noord".to_string(),
        "polder_midden".to_string(),
        0.5,
        1.5,
    )?)?;

    topologie.voeg_verbinding_toe(Verbinding::nieuw_gemaal(
        "gemaal_mz".to_string(),
        "polder_midden".to_string(),
        "polder_zuid".to_string(),
        0.4,
        1.2,
    )?)?;

    // Maak regenscenario met een duidelijke piek
    let mut regen_scenario = HashMap::new();

    // Regenpatroon: eerst droog, dan zware bui, dan afnemend
    let regen_patroon = vec
![
        0.0, 0.0, 0.0, 5.0, 10.0, 20.0, 30.0, 25.0, 15.0, 10.0,
        5.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0,
    ];

    for id in &["polder_noord", "polder_midden", "polder_zuid"] {
        regen_scenario.insert(id.to_string(), regen_patroon.clone());
    }

    // Voer simulatie uit
    let strategy = SimpeleUitstroomStrategy;
    let resultaat = run_netwerksimulatie(&topologie, &regen_scenario, 24, &strategy)?;

    Ok(resultaat)
}
