//! PDF document parsing en extractie module.
//!
//! Deze module biedt functionaliteit voor het parsen van PDF documenten
//! met optionele OCR ondersteuning voor gescande documenten.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod ocr;
pub mod parser;
pub mod types;

pub use ocr::{OcrBackend, OcrConfig};
pub use parser::{PdfDocument, PdfParser, TableExtractionConfig};
pub use types::{DocumentChunk, DocumentMetadata, Table, TableCell};

/// Error types voor PDF parsing.
#[derive(Error, Debug)]
pub enum PdfError {
    /// Bestand niet gevonden
    #[error("bestand niet gevonden: {0}")]
    FileNotFound(String),

    /// Ongeldig PDF formaat
    #[error("ongeldig PDF formaat: {0}")]
    InvalidPdfFormat(String),

    /// OCR fout
    #[error("OCR fout: {0}")]
    OcrError(String),

    /// Extractie fout
    #[error("extractie fout: {0}")]
    ExtractionError(String),

    /// IO fout
    #[error("IO fout: {0}")]
    IoError(#[from] std::io::Error),

    /// Parse fout
    #[error("parse fout: {0}")]
    ParseError(String),
}

/// Resultaat type voor PDF operaties.
pub type Result<T> = std::result::Result<T, PdfError>;

/// Configuratie voor de document parser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentConfig {
    /// OCR configuratie
    pub ocr: OcrConfig,

    /// Of offset (positie) informatie bijgehouden moet worden
    pub include_offsets: bool,

    /// Maximum grootte van een tekst chunk in karakters
    pub max_chunk_size: usize,

    /// Overlap tussen chunks in karakters
    pub chunk_overlap: usize,

    /// Of tabellen geëxtraheerd moeten worden
    pub extract_tables: bool,
}

impl Default for DocumentConfig {
    fn default() -> Self {
        Self {
            ocr: OcrConfig::default(),
            include_offsets: true,
            max_chunk_size: 1000,
            chunk_overlap: 100,
            extract_tables: true,
        }
    }
}

/// Geëxtraheerd document met tekst en metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    /// Unieke identifier
    pub id: uuid::Uuid,

    /// Originele bestandsnaam
    pub filename: String,

    /// Document metadata
    pub metadata: DocumentMetadata,

    /// Geëxtraheerde tekst chunks
    pub chunks: Vec<DocumentChunk>,

    /// Geëxtraheerde tabellen (indien beschikbaar)
    pub tables: Vec<Table>,

    /// Volledige tekst (samengevoegd uit alle chunks)
    pub full_text: String,

    /// Tijdstip van extractie
    pub extracted_at: DateTime<Utc>,
}

impl ExtractedDocument {
    /// Maak een nieuw extracted document.
    pub fn new(filename: String, full_text: String) -> Self {
        let id = uuid::Uuid::new_v4();
        let now = Utc::now();

        Self {
            id,
            metadata: DocumentMetadata::default(),
            chunks: Vec::new(),
            tables: Vec::new(),
            filename,
            full_text,
            extracted_at: now,
        }
    }

    /// Voeg een chunk toe aan het document.
    pub fn add_chunk(&mut self, chunk: DocumentChunk) {
        self.chunks.push(chunk);
    }

    /// Voeg een tabel toe aan het document.
    pub fn add_table(&mut self, table: Table) {
        self.tables.push(table);
    }

    /// Zet metadata voor het document.
    pub fn with_metadata(mut self, metadata: DocumentMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Split tekst in chunks op basis van de configuratie.
    pub fn chunk_text(&mut self, config: &DocumentConfig) {
        let text = &self.full_text;
        let chunk_size = config.max_chunk_size;
        let overlap = config.chunk_overlap;

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = (start + chunk_size).min(text.len());

            // Zoek laatste spatie om woorden niet af te breken
            let actual_end = if end < text.len() {
                text[start..end]
                    .rfind(' ')
                    .map(|pos| start + pos + 1)
                    .unwrap_or(end)
            } else {
                end
            };

            let chunk_text = text[start..actual_end].trim().to_string();

            if !chunk_text.is_empty() {
                chunks.push(DocumentChunk {
                    index: chunks.len(),
                    text: chunk_text,
                    page: None,
                    offset: Some(start as u32),
                    length: Some((actual_end - start) as u32),
                });
            }

            start = actual_end;

            // Voeg overlap toe
            if start < text.len() && overlap > 0 {
                start = start.saturating_sub(overlap);
            }
        }

        self.chunks = chunks;
    }

    /// Zoek tekst in het document.
    pub fn search(&self, query: &str) -> Vec<&DocumentChunk> {
        let query_lower = query.to_lowercase();
        self.chunks
            .iter()
            .filter(|chunk| chunk.text.to_lowercase().contains(&query_lower))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text() {
        let mut doc = ExtractedDocument::new(
            "test.pdf".to_string(),
            "Dit is een test document met meerdere zinnen. ".repeat(10),
        );

        let config = DocumentConfig {
            max_chunk_size: 100,
            chunk_overlap: 20,
            ..Default::default()
        };

        doc.chunk_text(&config);

        assert!(!doc.chunks.is_empty());
        assert!(doc.chunks.len() > 1);
    }

    #[test]
    fn test_search() {
        let mut doc = ExtractedDocument::new(
            "test.pdf".to_string(),
            "Dit is een test document met meerdere zinnen. Hier staat nog meer tekst.".to_string(),
        );

        doc.chunk_text(&DocumentConfig::default());

        let results = doc.search("test");
        assert!(!results.is_empty());
    }
}
