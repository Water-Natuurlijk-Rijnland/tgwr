//! Parse alle PDF bestanden in een directory en verzamel tekst en metadata.

use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let pdf_dir = if args.len() > 1 {
        &args[1]
    } else {
        "./peilbesluiten"
    };

    println!("=== PDF Batch Parser ===");
    println!("Directory: {}\n", pdf_dir);

    let path = Path::new(pdf_dir);

    if !path.exists() {
        println!("Directory '{}' bestaat niet.", pdf_dir);
        println!("Gebruik: cargo run --example batch_parse <directory>");
        println!("\nTip: Maak eerst een directory en plaats PDF's erin:");
        println!("  mkdir {}", pdf_dir);
        println!("  cp ~/Downloads/*.pdf {}", pdf_dir);
        return Ok(());
    }

    // Zoek alle PDF bestanden
    let pdf_files = find_pdf_files(path)?;

    if pdf_files.is_empty() {
        println!("Geen PDF bestanden gevonden in '{}'", pdf_dir);
        return Ok(());
    }

    println!("{} PDF bestanden gevonden:\n", pdf_files.len());

    // Parser initialiseren
    let parser = peilbeheer_documenten::PdfParser::new();

    let mut results = Vec::new();
    let mut total_chars = 0;

    for (index, pdf_path) in pdf_files.iter().enumerate() {
        println!("[{}/{}] Parsing: {}", index + 1, pdf_files.len(),
            pdf_path.file_name().unwrap_or_default().to_string_lossy());

        match parser.parse_file(pdf_path) {
            Ok(doc) => {
                total_chars += doc.full_text.len();

                println!("  ✓ Titel: {:?}", doc.metadata.title.as_ref().unwrap_or(&"(geen)".to_string()));
                println!("  ✓ Auteur: {:?}", doc.metadata.author.as_ref().unwrap_or(&"(onbekend)".to_string()));
                println!("  ✓ Tekst lengte: {} karakters", doc.full_text.len());
                println!("  ✓ Chunks: {}", doc.chunks.len());

                // Sla op in results
                results.push(ParsedDocument {
                    filename: doc.filename.clone(),
                    title: doc.metadata.title,
                    author: doc.metadata.author,
                    text_length: doc.full_text.len(),
                    chunk_count: doc.chunks.len(),
                    full_text: doc.full_text,
                });
            }
            Err(e) => {
                println!("  ✗ Fout: {}", e);
            }
        }
        println!();
    }

    // Samenvatting
    println!("=== Samenvatting ===");
    println!("Totaal verwerkt: {} PDF's", results.len());
    println!("Totaal karakters: {}", total_chars);

    // Exporteer naar JSON
    let json_output = serde_json::to_string_pretty(&results)?;
    fs::write("peilbesluiten_data.json", &json_output)?;
    println!("\nGeëxporteerd naar: peilbesluiten_data.json");

    // Exporteer gecombineerde tekst
    let all_text: String = results.iter()
        .map(|d| format!("=== {} ===\n{}", d.filename, d.full_text))
        .collect::<Vec<_>>()
        .join("\n\n");
    fs::write("peilbesluiten_text.txt", &all_text)?;
    println!("Geëxporteerd naar: peilbesluiten_text.txt");

    Ok(())
}

fn find_pdf_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut pdfs = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "pdf" {
                    pdfs.push(path);
                }
            }
        }
    }

    pdfs.sort();
    Ok(pdfs)
}

#[derive(serde::Serialize)]
struct ParsedDocument {
    filename: String,
    title: Option<String>,
    author: Option<String>,
    text_length: usize,
    chunk_count: usize,
    full_text: String,
}
