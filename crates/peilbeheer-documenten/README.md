# peilbeheer-documenten

PDF document parsing en extractie module voor Peilbeheer HHVR.

Deze module biedt functionaliteit voor het parsen van PDF documenten met optionele OCR ondersteuning voor gescande documenten.

## Kenmerken

- **Tekst extractie** uit PDF's met embedded tekst (snel, geen OCR nodig)
- **OCR ondersteuning** via Tesseract, EasyOCR of PaddleOCR
- **Automatische chunking** van lange documenten
- **Metadata extractie** (titel, auteur, etc.)
- **Zoekfunctionaliteit** in geëxtraheerde tekst

## Gebruik

### Basis tekst extractie

```rust
use peilbeheer_documenten::parser::extract_text;

let text = extract_text("document.pdf")?;
println!("Extracted text: {}", text);
```

### Met OCR voor gescande documenten

```rust
use peilbeheer_documenten::parser::extract_text_ocr;

// Voor Engelse documenten
let text = extract_text_ocr("scanned.pdf", "eng")?;

// Voor Nederlandse documenten
let text = extract_text_ocr("scanned.pdf", "nld")?;
```

### Geavanceerd gebruik met PdfParser

```rust
use peilbeheer_documenten::{parser::PdfParser, ocr::{OcrConfig, OcrBackend}};

// Configureer OCR
let ocr_config = OcrConfig::new(OcrBackend::Tesseract, "nld")
    .with_offsets(true);

// Parse document
let parser = PdfParser::new().with_ocr_config(ocr_config);
let doc = parser.parse_file("document.pdf")?;

// Toon metadata
println!("Titel: {:?}", doc.metadata.title);
println!("Aantal pagina's: {:?}", doc.metadata.page_count);

// Zoek in document
for chunk in doc.search("waterpeil") {
    println!("Gevonden: {}", chunk.text);
}

// Itereer over chunks
for chunk in &doc.chunks {
    println!("Chunk {}: {}", chunk.index, chunk.text);
}
```

### PdfDocument wrapper

```rust
use peilbeheer_documenten::parser::PdfDocument;

let doc = PdfDocument::open("document.pdf")?;

// Snel tekst extractie
let text = doc.text()?;

// Of met OCR
let text = doc.text_with_ocr("eng")?;

// Volledig document met chunks
let extracted = doc.parse()?;
```

## OCR Backend

De module ondersteunt meerdere OCR backends via de kreuzberg library:

| Backend | Beschrijving | Vereist |
|---------|-------------|---------|
| Tesseract | Meest gebruikte OCR engine, ondersteunt veel talen | Tesseract binary geïnstalleerd |
| EasyOCR | Deep learning gebaseerd, Python dependency | Python, EasyOCR |
| PaddleOCR | Chinese OCR engine, ondersteunt 80+ talen | PaddleOCR |

### Tesseract installeren

```bash
# macOS
brew install tesseract tesseract-lang

# Ubuntu/Debian
apt-get install tesseract-ocr tesseract-ocr-nld

# Windows
# Download van: https://github.com/UB-Mannheim/tesseract/wiki
```

## Types

### ExtractedDocument

```rust
pub struct ExtractedDocument {
    pub id: uuid::Uuid,
    pub filename: String,
    pub metadata: DocumentMetadata,
    pub chunks: Vec<DocumentChunk>,
    pub tables: Vec<Table>,
    pub full_text: String,
    pub extracted_at: DateTime<Utc>,
}
```

### DocumentChunk

```rust
pub struct DocumentChunk {
    pub index: usize,
    pub text: String,
    pub page: Option<u32>,
    pub offset: Option<u32>,
    pub length: Option<u32>,
}
```

### DocumentMetadata

```rust
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub creation_date: Option<DateTime<Utc>>,
    pub modification_date: Option<DateTime<Utc>>,
    pub page_count: Option<u32>,
    pub pdf_version: Option<String>,
    // ... extra velden
}
```

## Foutafhandeling

```rust
use peilbeheer_documenten::PdfError;

match extract_text("document.pdf") {
    Ok(text) => println!("{}", text),
    Err(PdfError::FileNotFound(path)) => {
        eprintln!("Bestand niet gevonden: {}", path);
    }
    Err(PdfError::OcrError(msg)) => {
        eprintln!("OCR fout: {}", msg);
    }
    Err(e) => eprintln!("Andere fout: {}", e),
}
```

## Dependencies

- `kreuzberg` - Document extraction library met OCR ondersteuning
- `pdf-extract` - Snelle tekst extractie voor PDF's met embedded tekst
