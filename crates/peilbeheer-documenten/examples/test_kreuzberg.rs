//! Test PDF parsing met kreuzberg voor volledige metadata extractie.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kreuzberg PDF Parsing Test ===\n");

    // Gebruik kreuzberg direct voor volledige metadata
    let result = kreuzberg::extract_file_sync(
        "test_document.pdf",
        None,
        &kreuzberg::ExtractionConfig::default(),
    )?;

    println!("Content (eerste 300 karakters):");
    println!("{}\n", &result.content.chars().take(300).collect::<String>());

    println!("Metadata:");
    let metadata = result.metadata;
    println!("  Titel: {:?}", metadata.title);
    println!("  Onderwerp: {:?}", metadata.subject);
    println!("  Auteurs: {:?}", metadata.authors);
    println!("  Trefwoorden: {:?}", metadata.keywords);
    println!("  Taal: {:?}", metadata.language);

    Ok(())
}
