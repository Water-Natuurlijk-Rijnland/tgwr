//! Voorbeeld programma om PDF parsing te testen.

use peilbeheer_documenten::{parser::extract_text, ocr::{OcrConfig, OcrBackend}, parser::PdfParser};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Maak een simpele test PDF met lorem ipsum
    let test_pdf_path = "test_document.pdf";

    // Probeer de PDF te parsen
    println!("=== PDF Parsing Test ===\n");

    match extract_text(test_pdf_path) {
        Ok(text) => {
            println!("Succesvol tekst geëxtraheerd!");
            println!("Aantal karakters: {}", text.len());
            println!("\n--- Eerste 500 karakters ---\n{}", &text.chars().take(500).collect::<String>());
            println!("\n========================\n");

            // Test met parser voor metadata
            let parser = PdfParser::new();
            match parser.parse_file(test_pdf_path) {
                Ok(doc) => {
                    println!("Document metadata:");
                    println!("  ID: {}", doc.id);
                    println!("  Bestandsnaam: {}", doc.filename);
                    println!("  Titel: {:?}", doc.metadata.title);
                    println!("  Auteur: {:?}", doc.metadata.author);
                    println!("  Pagina's: {:?}", doc.metadata.page_count);
                    println!("  Aantal chunks: {}", doc.chunks.len());
                    println!("  Geëxtraheerd op: {}", doc.extracted_at);

                    // Test zoekfunctionaliteit
                    let search_results = doc.search("lorem");
                    println!("\nZoekresultaten voor 'lorem': {} gevonden", search_results.len());
                }
                Err(e) => {
                    println!("Parser error: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Fout bij tekst extractie: {}", e);
            println!("\nInfo: Om een echte test uit te voeren, plaats een PDF bestand als '{}' in de project root.", test_pdf_path);
        }
    }

    Ok(())
}
