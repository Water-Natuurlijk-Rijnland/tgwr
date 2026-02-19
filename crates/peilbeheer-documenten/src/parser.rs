//! PDF parser implementatie.

use super::ocr::OcrConfig;
use super::{ExtractedDocument, PdfError, Result};
use std::fs;
use std::path::Path;

/// Configuratie voor tabel extractie.
#[derive(Debug, Clone)]
pub struct TableExtractionConfig {
    /// Of tabellen geëxtraheerd moeten worden
    pub enabled: bool,

    /// Minimum aantal rijen voor een tabel
    pub min_rows: usize,

    /// Minimum aantal kolommen voor een tabel
    pub min_columns: usize,
}

impl Default for TableExtractionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_rows: 2,
            min_columns: 2,
        }
    }
}

/// PDF parser voor het extraheren van tekst en tabellen.
pub struct PdfParser {
    ocr_config: OcrConfig,
    table_config: TableExtractionConfig,
}

impl PdfParser {
    /// Maak een nieuwe PDF parser.
    pub fn new() -> Self {
        Self {
            ocr_config: OcrConfig::default(),
            table_config: TableExtractionConfig::default(),
        }
    }

    /// Maak een parser met OCR configuratie.
    pub fn with_ocr_config(mut self, config: OcrConfig) -> Self {
        self.ocr_config = config;
        self
    }

    /// Maak een parser met tabel extractie configuratie.
    pub fn with_table_config(mut self, config: TableExtractionConfig) -> Self {
        self.table_config = config;
        self
    }

    /// Parse een PDF bestand en extraheer tekst.
    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> Result<ExtractedDocument> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(PdfError::FileNotFound(path.display().to_string()));
        }

        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown.pdf".to_string());

        match self.ocr_config.backend {
            super::ocr::OcrBackend::None => self.parse_without_ocr(path, &filename),
            _ => self.parse_with_ocr(path, &filename),
        }
    }

    /// Parse PDF zonder OCR (sneller, werkt alleen voor embedded tekst).
    fn parse_without_ocr(&self, path: &Path, filename: &str) -> Result<ExtractedDocument> {
        let bytes = fs::read(path)?;

        // Gebruik pdf-extract voor tekst extractie
        let text = pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| PdfError::ExtractionError(e.to_string()))?;

        let mut doc = ExtractedDocument::new(filename.to_string(), text);
        doc.chunk_text(&super::DocumentConfig::default());

        Ok(doc)
    }

    /// Parse PDF met OCR voor gescande documenten.
    fn parse_with_ocr(&self, path: &Path, filename: &str) -> Result<ExtractedDocument> {
        // Gebruik kreuzberg voor OCR extractie
        let ocr_config = self.ocr_config.as_kreuzberg_ocr_config();
        let extraction_config = kreuzberg::ExtractionConfig {
            ocr: ocr_config,
            ..Default::default()
        };

        let result = kreuzberg::extract_file_sync(path, None, &extraction_config)
            .map_err(|e: kreuzberg::KreuzbergError| PdfError::OcrError(e.to_string()))?;

        let full_text = result.content;
        let mut doc = ExtractedDocument::new(filename.to_string(), full_text.clone());

        // Extraheer metadata
        let metadata = result.metadata;
        if let Some(title) = metadata.title {
            doc.metadata.title = Some(title);
        }
        if let Some(authors) = metadata.authors
            && !authors.is_empty() {
                doc.metadata.author = Some(authors.join(", "));
            }

        // Chunk de tekst
        doc.chunk_text(&super::DocumentConfig::default());

        Ok(doc)
    }

    /// Parse meerdere PDF bestanden.
    pub fn parse_batch<P: AsRef<Path>>(&self, paths: &[P]) -> Vec<Result<ExtractedDocument>> {
        paths
            .iter()
            .map(|path| self.parse_file(path))
            .collect()
    }
}

impl Default for PdfParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functie voor simpele PDF extractie.
///
/// # Example
///
/// ```no_run
/// use peilbeheer_documenten::parser::extract_text;
///
/// let text = extract_text("document.pdf").unwrap();
/// println!("Extracted text: {}", text);
/// ```
pub fn extract_text<P: AsRef<Path>>(path: P) -> Result<String> {
    let parser = PdfParser::new();
    let doc = parser.parse_file(path)?;
    Ok(doc.full_text)
}

/// Convenience functie voor PDF extractie met OCR.
///
/// # Example
///
/// ```no_run
/// use peilbeheer_documenten::parser::extract_text_ocr;
///
/// let text = extract_text_ocr("scanned.pdf", "eng").unwrap();
/// println!("Extracted text: {}", text);
/// ```
pub fn extract_text_ocr<P: AsRef<Path>>(path: P, language: &str) -> Result<String> {
    let ocr_config = super::ocr::OcrConfig::new(super::ocr::OcrBackend::Tesseract, language);
    let parser = PdfParser::new().with_ocr_config(ocr_config);
    let doc = parser.parse_file(path)?;
    Ok(doc.full_text)
}

/// PDF document wrapper voor parsing.
pub struct PdfDocument {
    path: std::path::PathBuf,
}

impl PdfDocument {
    /// Open een PDF document.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(PdfError::FileNotFound(path.display().to_string()));
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Extraheer tekst uit het document.
    pub fn text(&self) -> Result<String> {
        extract_text(&self.path)
    }

    /// Extraheer tekst met OCR.
    pub fn text_with_ocr(&self, language: &str) -> Result<String> {
        extract_text_ocr(&self.path, language)
    }

    /// Parse als volledig ExtractedDocument.
    pub fn parse(&self) -> Result<ExtractedDocument> {
        let parser = PdfParser::new();
        parser.parse_file(&self.path)
    }

    /// Parse met OCR configuratie.
    pub fn parse_with_config(&self, ocr_config: OcrConfig) -> Result<ExtractedDocument> {
        let parser = PdfParser::new().with_ocr_config(ocr_config);
        parser.parse_file(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_parser_default() {
        let parser = PdfParser::new();
        assert_eq!(parser.ocr_config.backend, super::super::ocr::OcrBackend::None);
    }

    #[test]
    fn test_pdf_parser_with_ocr() {
        let ocr_config = OcrConfig::new(super::super::ocr::OcrBackend::Tesseract, "eng");
        let parser = PdfParser::new().with_ocr_config(ocr_config);
        assert_eq!(parser.ocr_config.language, "eng");
    }

    #[test]
    fn test_pdf_document_nonexistent() {
        let result = PdfDocument::open("/nonexistent/file.pdf");
        assert!(result.is_err());
    }
}
