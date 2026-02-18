//! Maak een simpele test PDF voor parsing tests met lopdf 0.34+

use lopdf::{Document, Object, dictionary, Stream};
use lopdf::content::{Content, Operation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::with_version("1.5");

    // Pages zijn het root element
    let pages_id = doc.new_object_id();

    // Font
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    // Maak content operations
    let operations = vec![
        Operation::new("BT", vec![]),  // Begin Text
        Operation::new("Tf", vec!["F1".into(), 12.into()]),  // Font and size
        Operation::new("Td", vec![100.into(), 700.into()]),  // Position
        Operation::new("Tj", vec![Object::string_literal("Peilbeheer HHVR - Test Document")]),  // Text
        Operation::new("ET", vec![]),  // End Text
        Operation::new("BT", vec![]),
        Operation::new("Td", vec![100.into(), 670.into()]),
        Operation::new("Tj", vec![Object::string_literal("====================================")]),
        Operation::new("ET", vec![]),
        Operation::new("BT", vec![]),
        Operation::new("Td", vec![100.into(), 640.into()]),
        Operation::new("Tj", vec![Object::string_literal("Dit is een test PDF document voor de")]),
        Operation::new("ET", vec![]),
        Operation::new("BT", vec![]),
        Operation::new("Td", vec![100.into(), 625.into()]),
        Operation::new("Tj", vec![Object::string_literal("peilbeheer-documenten crate.")]),
        Operation::new("ET", vec![]),
        Operation::new("BT", vec![]),
        Operation::new("Td", vec![100.into(), 600.into()]),
        Operation::new("Tj", vec![Object::string_literal("")]),
        Operation::new("ET", vec![]),
        Operation::new("BT", vec![]),
        Operation::new("Td", vec![100.into(), 575.into()]),
        Operation::new("Tj", vec![Object::string_literal("Lorem ipsum dolor sit amet, consectetur")]),
        Operation::new("ET", vec![]),
        Operation::new("BT", vec![]),
        Operation::new("Td", vec![100.into(), 560.into()]),
        Operation::new("Tj", vec![Object::string_literal("adipiscing elit. Sed do eiusmod tempor")]),
        Operation::new("ET", vec![]),
        Operation::new("BT", vec![]),
        Operation::new("Td", vec![100.into(), 545.into()]),
        Operation::new("Tj", vec![Object::string_literal("incididunt ut labore et dolore magna aliqua.")]),
        Operation::new("ET", vec![]),
    ];

    let content = Content {
        operations,
    };

    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));

    // Pagina resources
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    // Pagina object
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],  // A4
    });

    // Pages dictionary
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    // Document Info dictionary (metadata)
    let info_id = doc.add_object(dictionary! {
        "Title" => Object::string_literal("Peilbeheer Test Document"),
        "Author" => Object::string_literal("Peilbeheer HHVR Team"),
        "Subject" => Object::string_literal("PDF parsing test"),
        "Keywords" => Object::string_literal("test, pdf, peilbeheer, water"),
        "Creator" => Object::string_literal("peilbeheer-documenten crate"),
        "Producer" => Object::string_literal("lopdf"),
    });

    // Catalog
    let catalog_id = doc.new_object_id();
    let catalog = dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    };
    doc.objects.insert(catalog_id, Object::Dictionary(catalog));

    doc.trailer.set("Root", catalog_id);
    doc.trailer.set("Info", info_id);

    // Sla de PDF op
    doc.save("test_document.pdf")?;

    println!("Test PDF aangemaakt: test_document.pdf");
    println!("  - Titel: Peilbeheer Test Document");
    println!("  - Auteur: Peilbeheer HHVR Team");
    println!("  - Onderwerp: PDF parsing test");
    println!("  - Keywords: test, pdf, peilbeheer, water");
    Ok(())
}
