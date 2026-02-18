// Voorbeeld van CSV/JSON export.
//
// Dit voorbeeld demonstreert:
// - Export van simulatieresultaten naar CSV
// - Export van simulatieresultaten naar JSON
// - Per-peilgebied export
// - Statistieken berekenen en exporteren

use peilbeheer_simulatie::export::*;
use peilbeheer_simulatie::netwerk::*;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CSV/JSON Export Voorbeeld ===\n");

    // Stap 1: Maak en voer een scenario uit
    println!("1. Scenario uitvoeren...");
    let resultaat = voer_scenario()?;

    println!(
        "   Simulatie voltooid: {} tijdstappen\n",
        resultaat.tijdstappen.len()
    );

    // Stap 2: CSV export
    println!("2. CSV export (eerste 200 karakters):");
    let csv_export = CsvExport::nieuw();
    let csv = csv_export.als_string(&resultaat)?;
    let preview = csv.chars().take(200).collect::<String>();
    println!("{}\n", preview);

    // Stap 3: CSV export naar bestand
    println!("3. CSV opslaan naar bestand...");
    csv_export.naar_bestand(&resultaat, "simulatie_resultaat.csv")?;
    println!("   Opgeslagen: simulatie_resultaat.csv");

    // Stap 4: JSON export
    println!("\n4. JSON export (eerste 200 karakters):");
    let json_export = JsonExport::nieuw();
    let json = json_export.als_pretty_string(&resultaat)?;
    let preview = json.chars().take(200).collect::<String>();
    println!("{}\n", preview);

    // Stap 5: JSON export naar bestand
    println!("5. JSON opslaan naar bestand...");
    json_export.naar_bestand(&resultaat, "simulatie_resultaat.json")?;
    println!("   Opgeslagen: simulatie_resultaat.json");

    // Stap 6: Per-peilgebied JSON export
    println!("\n6. Per-peilgebied JSON export:");
    let per_peilgebied = json_export.per_peilgebied(&resultaat)?;

    for (id, data) in &per_peilgebied {
        println!("   - {}: {} tijdstappen", id, data.tijdstappen.len());
    }

    // Stap 7: Statistieken
    println!("\n7. Statistieken:");
    let stats = bereken_statistieken(&resultaat)?;

    println!("   Aantal tijdstappen: {}", stats.aantal_tijdstappen);
    println!("   Totale tijd: {} minuten", stats.totale_tijd);

    for (id, stat) in &stats.peilgebieden {
        println!(
            "\n   {}:",
            id
        );
        println!("     Min waterstand: {:.3} m", stat.min_waterstand);
        println!("     Max waterstand: {:.3} m", stat.max_waterstand);
        println!("     Gem waterstand: {:.3} m", stat.gem_waterstand);
        println!("     Totale uitstroom: {:.3} m³", stat.totale_uitstroom);
        println!("     Pomp-uren: {:.2} uur", stat.pomp_uren);
        println!("     Gem regen: {:.2} mm/u", stat.gem_regen);
    }

    // Stap 8: Statistieken JSON export
    println!("\n8. Statistieken JSON opslaan...");
    let stats_json = statistieken_als_json(&resultaat)?;
    std::fs::write("simulatie_statistieken.json", &stats_json)?;
    println!("   Opgeslagen: simulatie_statistieken.json");

    println!("\n=== Export voltooid ===");
    println!("\nOpgeslagen bestanden:");
    println!("  - simulatie_resultaat.csv");
    println!("  - simulatie_resultaat.json");
    println!("  - simulatie_statistieken.json");

    Ok(())
}

fn voer_scenario() -> Result<NetwerkSimulatieResultaat, Box<dyn std::error::Error>> {
    // Maak topologie
    let mut topologie = NetwerkTopologie::nieuw();

    let peilgebieden = vec![
        ("polder_noord", "Noordelijke Polder", 200_000.0, -0.50),
        ("polder_zuid", "Zuidelijke Polder", 150_000.0, -0.55),
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

    // Gemaalverbinding
    topologie.voeg_verbinding_toe(Verbinding::nieuw_gemaal(
        "gemaal".to_string(),
        "polder_noord".to_string(),
        "polder_zuid".to_string(),
        0.5,
        1.5,
    )?)?;

    // Maak regenscenario
    let mut regen_scenario = HashMap::new();
    regen_scenario.insert(
        "polder_noord".to_string(),
        vec
![
            0.0, 0.0, 5.0, 15.0, 25.0, 20.0, 10.0, 5.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
        ],
    );
    regen_scenario.insert(
        "polder_zuid".to_string(),
        vec
![
            0.0, 0.0, 4.0, 12.0, 22.0, 18.0, 9.0, 4.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
        ],
    );

    // Voer simulatie uit
    let strategy = SimpeleUitstroomStrategy;
    let resultaat = run_netwerksimulatie(&topologie, &regen_scenario, 24, &strategy)?;

    Ok(resultaat)
}
