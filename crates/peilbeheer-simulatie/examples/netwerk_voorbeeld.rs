// Voorbeeld van multi-peilgebied netwerksimulatie
//
// Dit voorbeeld demonstreert:
// - Het opzetten van een netwerk met meerdere peilgebieden
// - Different verbindingstypes (gemaal, overstort, keerklep)
// - Het simuleren van waterstromen tijdens een regen_event
// - Resultaatanalyse

use peilbeheer_simulatie::netwerk::*;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Multi-Peilgebied Netwerksimulatie ===\n");

    // Stap 1: Maak netwerktopologie
    let topologie = maak_topologie()?;
    println!("Topologie aangemaakt met {} peilgebieden", topologie.peilgebieden.len());
    println!("Verbindingen: {}", topologie.verbindingen.len());

    // Stap 2: Valideer topologie
    topologie.valideer()?;
    println!("Topologie geldig\n");

    // Stap 3: Start simulatie
    let mut simulatie = NetwerkSimulatie::nieuw(topologie.clone())?;

    // Toon startcondities
    println!("Startcondities:");
    for (id, ws) in &simulatie.waterstanden {
        let config = &topologie.peilgebieden[id];
        println!("  - {}: waterstand={:.2} m, streefpeil={:.2} m",
            id, ws, config.streefpeil);
    }
    println!();

    // Stap 4: Simuleer regengebeurtenis
    println!("Simuleer regengebeurtenis (6 uur)...\n");

    let strategy = SimpeleUitstroomStrategy;
    let mut totale_kosten = 0.0;

    for uur in 0..6 {
        println!("--- Uur {} ---", uur);

        // Regenintensiteit: hoog in eerste 3 uur, daarna afnemend
        let regen_intensiteit = match uur {
            0..=2 => 10.0,  // Zware regen
            3..=4 => 5.0,   // Matige regen
            _ => 0.0,       // Droog
        };

        println!("Regenintensiteit: {:.1} mm/uur", regen_intensiteit);

        // Simuleer 60 minuten
        for minuut in 0..60 {
            let mut regen = HashMap::new();
            for id in topologie.peilgebieden.keys() {
                // Iets minder regen in lagere polders (effect van berging)
                let factor = if id == "polder_hoog" { 1.0 }
                             else if id == "polder_midden" { 0.9 }
                             else { 0.8 };
                regen.insert(id.clone(), regen_intensiteit * factor);
            }

            let statussen = simulatie.simuleer_stap(&regen, &strategy)?;

            // Bereken pompkosten
            for status in &statussen {
                if status.pomp_actief {
                    // Eenvoudige kostberekening: €0.15 per kWh
                    let power_kw = status.uitstroom_debiet * 9.81 * 2.0 / 0.7; // P = ρ*g*Q*H/η
                    let kosten = power_kw * 0.15 / 60.0; // Per minuut
                    totale_kosten += kosten;
                }
            }

            // Print elke 15 minuten
            if minuut % 15 == 0 && minuut > 0 {
                println!("  Minuut {}:", minuut);
                for status in &statussen {
                    let config = &topologie.peilgebieden[&status.id];
                    let afwijking = status.waterstand - config.streefpeil;
                    println!("    {}: ws={:.3} m (afwijking={:+.2} cm), pomp={}",
                        status.id,
                        status.waterstand,
                        afwijking * 100.0,
                        if status.pomp_actief { "AAN" } else { "uit" }
                    );
                }
            }
        }

        // Toon verbindingstromen
        let stromen = simulatie.bereken_stromen(&HashMap::new())?;
        println!("  Verbindingstromen:");
        for stroom in &stromen {
            if stroom.actief {
                let verbinding = &topologie.verbindingen[&stroom.verbinding_id];
                println!("    {} -> {}: {:.3} m³/s ({:.0}% capaciteit)",
                    verbinding.van_id,
                    verbinding.naar_id,
                    stroom.debiet,
                    stroom.benutting * 100.0
                );
            }
        }
        println!();
    }

    // Stap 5: Resultaat
    println!("=== Resultaat ===");
    println!("Totale pompkosten: €{:.2}", totale_kosten);

    println!("\nEindsituatie:");
    for (id, ws) in &simulatie.waterstanden {
        let config = &topologie.peilgebieden[id];
        let afwijking = ws - config.streefpeil;
        let status = if config.is_waterstand_geldig(*ws) {
            "OK"
        } else {
            "BOVEN MAYPEIL!"
        };
        println!("  - {}: waterstand={:.3} m, afwijking={:+.2} cm [{}]",
            id, ws, afwijking * 100.0, status);
    }

    Ok(())
}

fn maak_topologie() -> Result<NetwerkTopologie, NetwerkFout> {
    let mut topologie = NetwerkTopologie::nieuw();

    // Drie peilgebieden in cascade configuratie
    let peilgebieden = vec![
        ("polder_hoog", "Hoogland", 150_000.0, -0.60),
        ("polder_midden", "Middengebied", 120_000.0, -0.65),
        ("polder_laag", "Laagland", 100_000.0, -0.70),
    ];

    for (id, naam, oppervlakte, streefpeil) in peilgebieden {
        topologie.voeg_peilgebied_toe(PeilgebiedConfig {
            id: id.to_string(),
            naam: Some(naam.to_string()),
            oppervlakte,
            streefpeil,
            marge: 0.20,
            maaiveld_niveau: 0.0,
            max_uitstroom_debiet: 0.6,
            verdamping: 0.0,
            infiltratie: 0.0,
        })?;
    }

    // Gemaal van hoog naar midden
    topologie.voeg_verbinding_toe(
        Verbinding::nieuw_gemaal(
            "gemaal_hm".to_string(),
            "polder_hoog".to_string(),
            "polder_midden".to_string(),
            0.4,
            1.5,
        )?
    )?;

    // Overstort van midden naar laag (passief)
    topologie.voeg_verbinding_toe(
        Verbinding::nieuw_overstort(
            "overstort_ml".to_string(),
            "polder_midden".to_string(),
            "polder_laag".to_string(),
            0.5,
            -0.55, // Drempel iets boven streefpeil
        )?
    )?;

    Ok(topologie)
}
