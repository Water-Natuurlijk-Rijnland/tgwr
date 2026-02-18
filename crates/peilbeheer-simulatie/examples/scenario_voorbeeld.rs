// Voorbeeld van scenario beheer.
//
// Dit voorbeeld demonstreert:
// - Het maken van een scenario met de ScenarioBouwer
// - Scenario opslaan naar JSON bestand
// - Scenario laden uit JSON bestand
// - Scenario uitvoeren

use peilbeheer_simulatie::scenario::*;
use peilbeheer_simulatie::netwerk::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Scenario Beheer Voorbeeld ===\n");

    // Stap 1: Maak een scenario
    println!("1. Scenario maken...");
    let topologie = maak_topologie()?;
    let scenario = maak_scenario(topologie)?;

    println!("   Scenario ID: {}", scenario.id);
    println!("   Naam: {:?}", scenario.naam);
    println!("   Beschrijving: {:?}", scenario.beschrijving);
    println!("   Aantal peilgebieden: {}", scenario.topologie.peilgebieden.len());
    println!("   Simulatie duur: {} uur\n", scenario.parameters.duration_hours);

    // Stap 2: Sla scenario op
    let pad = PathBuf::from("voorbeeld_scenario.json");
    println!("2. Scenario opslaan naar {:?}...", pad);
    scenario.sla_op(&pad)?;
    println!("   Scenario opgeslagen!\n");

    // Stap 3: Laad scenario
    println!("3. Scenario laden...");
    let geladen = Scenario::laad(&pad)?;
    println!("   Scenario geladen: {}", geladen.id);
    println!("   Tags: {:?}\n", geladen.metadata.tags);

    // Stap 4: Toon JSON preview
    println!("4. JSON preview (eerste 500 karakters):");
    let json = geladen.als_json()?;
    let preview = json.chars().take(500).collect::<String>();
    println!("{}\n", preview);

    // Stap 5: Maak een kopie met nieuwe ID
    println!("5. Scenario kopiëren...");
    let kopie = geladen.kopie_met_id("scenario_kopie".to_string());
    println!("   Kopie ID: {}", kopie.id);
    println!("   Auteur: {:?}\n", kopie.metadata.auteur);

    // Stap 6: Voer scenario uit
    println!("6. Scenario uitvoeren...");
    let resultaat = voer_scenario_uit(&geladen)?;
    println!("   Simulatie voltooid!");
    println!("   Aantal tijdstappen: {}", resultaat.resultaat.tijdstappen.len());

    // Laatste tijdstap details
    if let Some(laatste) = resultaat.resultaat.tijdstappen.last() {
        println!("   Eindtijd: {} minuten", laatste.tijd);
        for (id, status) in &laatste.statussen {
            println!(
                "     - {}: waterstand={:.3} m, pomp={}",
                id,
                status.waterstand,
                if status.pomp_actief { "AAN" } else { "uit" }
            );
        }
    }

    // Stap 7: Sla resultaat op
    let resultaat_pad = PathBuf::from("voorbeeld_resultaat.json");
    println!("\n7. Resultaat opslaan naar {:?}...", resultaat_pad);
    resultaat.sla_op(&resultaat_pad)?;
    println!("   Resultaat opgeslagen!");

    println!("\n=== Voorbeeld voltooid ===");
    println!("\nOpgeslagen bestanden:");
    println!("  - {:?} (scenario)", pad);
    println!("  - {:?} (resultaat)", resultaat_pad);

    Ok(())
}

fn maak_topologie() -> Result<NetwerkTopologie, NetwerkFout> {
    let mut topologie = NetwerkTopologie::nieuw();

    // Drie peilgebieden
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

    Ok(topologie)
}

fn maak_scenario(topologie: NetwerkTopologie) -> Result<Scenario, ScenarioFout> {
    // Maak een regenscenario met een "design storm"
    let regen_noord = vec![
        0.0, 0.0, 5.0, 15.0, 25.0, 20.0, 10.0, 5.0, 2.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let regen_midden = vec![
        0.0, 0.0, 4.0, 12.0, 22.0, 18.0, 9.0, 4.0, 2.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let regen_zuid = vec![
        0.0, 0.0, 3.0, 10.0, 20.0, 16.0, 8.0, 4.0, 2.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];

    ScenarioBouwer::nieuw("voorbeeld_scenario".to_string())
        .met_naam("Voorbeeld Regen Scenario".to_string())
        .met_beschrijving(
            "Een regenscenario met een zomerse bui in de eerste 10 uur"
                .to_string(),
        )
        .met_topologie(topologie)
        .met_regen("polder_noord".to_string(), regen_noord)
        .met_regen("polder_midden".to_string(), regen_midden)
        .met_regen("polder_zuid".to_string(), regen_zuid)
        .met_regen_type(RegenscenarioType::Ontworpen)
        .met_duur(24)
        .met_strategy(StrategyType::Simpel)
        .met_auteur("Demo".to_string())
        .met_versie("1.0".to_string())
        .voeg_tag("demo".to_string())
        .voeg_tag("testen".to_string())
        .voeg_tag("regen".to_string())
        .bouw()
}

fn voer_scenario_uit(
    scenario: &Scenario,
) -> Result<ScenarioResultaat, Box<dyn std::error::Error>> {
    use peilbeheer_simulatie::netwerk::run_netwerksimulatie;

    // Bereid regen data voor
    let regen_scenario = &scenario.regen_scenario.regen_per_uur;

    // Kies strategy
    let strategy = SimpeleUitstroomStrategy;

    // Voer simulatie uit
    let resultaat = run_netwerksimulatie(
        &scenario.topologie,
        regen_scenario,
        scenario.parameters.duration_hours,
        &strategy,
    )?;

    Ok(ScenarioResultaat::nieuw(scenario.clone(), resultaat))
}
