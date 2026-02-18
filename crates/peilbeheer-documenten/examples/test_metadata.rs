//! Test PDF parsing met metadata extractie via kreuzberg.

use peilbeheer_documenten::{parser::PdfParser, ocr::{OcrConfig, OcrBackend}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PDF Metadata Extractie Test ===\n");

    // Gebruik OCR configuratie om kreuzberg te forceren (ook zonder echte OCR)
    let ocr_config = OcrConfig::new(OcrBackend::Tesseract, "eng");
    let parser = PdfParser::new().with_ocr_config(ocr_config);

    match parser.parse_file("test_document.pdf") {
        Ok(doc) => {
            println!("Succesvol document geparsed!");
            println!("\n--- Document Info ---");
            println!("  ID: {}", doc.id);
            println!("  Bestandsnaam: {}", doc.filename);
            println!("  Geëxtraheerd op: {}", doc.extracted_at);

            println!("\n--- Metadata ---");
            println!("  Titel: {:?}", doc.metadata.title);
            println!("  Auteur: {:?}", doc.metadata.author);
            println!("  Aantal pagina's: {:?}", doc.metadata.page_count);
            println!("  PDF versie: {:?}", doc.metadata.pdf_version);

            println!("\n--- Content ---");
            println!("  Volledige tekst lengte: {} karakters", doc.full_text.len());
            println!("  Aantal chunks: {}", doc.chunks.len());

            println!("\n--- Eerste 300 karakters ---");
            println!("{}", &doc.full_text.chars().take(300).collect::<String>());

            println!("\n--- Zoek test ---");
            let results = doc.search("peilbeheer");
            println!("  Zoekresultaten voor 'peilbeheer': {}", results.len());
            for chunk in results {
                println!("    - Chunk {}: {}", chunk.index, chunk.text.trim());
            }
        }
        Err(e) => {
            println!("Fout bij parsen: {}", e);
        }
    }

    Ok(())
}
