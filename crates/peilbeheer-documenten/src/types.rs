//! Algemene types voor document extractie.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata over een geëxtraheerd document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Titel van het document (indien beschikbaar)
    pub title: Option<String>,

    /// Auteur van het document (indien beschikbaar)
    pub author: Option<String>,

    /// Aanmaakdatum van het document
    pub creation_date: Option<DateTime<Utc>>,

    /// Wijzigingsdatum van het document
    pub modification_date: Option<DateTime<Utc>>,

    /// Aantal pagina's
    pub page_count: Option<u32>,

    /// PDF versie
    pub pdf_version: Option<String>,

    /// Extra metadata velden
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: None,
            author: None,
            creation_date: None,
            modification_date: None,
            page_count: None,
            pdf_version: None,
            extra: HashMap::new(),
        }
    }
}

/// Een tekst chunk uit een document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// Chunk index (volgorde in document)
    pub index: usize,

    /// De tekst van deze chunk
    pub text: String,

    /// Pagina nummer (indien beschikbaar)
    pub page: Option<u32>,

    /// Byte offset in het originele document
    pub offset: Option<u32>,

    /// Lengte van de chunk in bytes
    pub length: Option<u32>,
}

impl DocumentChunk {
    /// Maak een nieuwe tekst chunk.
    pub fn new(text: String) -> Self {
        Self {
            index: 0,
            text,
            page: None,
            offset: None,
            length: None,
        }
    }

    /// Maak een nieuwe chunk met metadata.
    pub fn with_metadata(mut self, index: usize, page: Option<u32>) -> Self {
        self.index = index;
        self.page = page;
        self
    }

    /// Zet de offset informatie.
    pub fn with_offset(mut self, offset: u32, length: u32) -> Self {
        self.offset = Some(offset);
        self.length = Some(length);
        self
    }
}

/// Een geëxtraheerde tabel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Tabel index
    pub index: usize,

    /// Pagina nummer waar de tabel staat
    pub page: Option<u32>,

    /// Aantal rijen
    pub row_count: usize,

    /// Aantal kolommen
    pub column_count: usize,

    /// Optionele kopteksten
    pub headers: Vec<String>,

    /// Tabel cellen (rij-major volgorde)
    pub cells: Vec<Vec<TableCell>>,

    /// Positie in het document (indien beschikbaar)
    pub bbox: Option<BoundingBox>,
}

impl Table {
    /// Maak een nieuwe tabel.
    pub fn new(row_count: usize, column_count: usize) -> Self {
        Self {
            index: 0,
            page: None,
            row_count,
            column_count,
            headers: Vec::new(),
            cells: vec![vec![TableCell::default(); column_count]; row_count],
            bbox: None,
        }
    }

    /// Maak een tabel met headers.
    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.column_count = headers.len();
        self.headers = headers;
        self
    }

    /// Zet een cel waarde.
    pub fn set_cell(&mut self, row: usize, col: usize, value: String) {
        if row < self.cells.len() && col < self.cells[row].len() {
            self.cells[row][col] = TableCell {
                text: value,
                row,
                col,
                merged: false,
            };
        }
    }

    /// Converteer de tabel naar JSON.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Een enkele cel in een tabel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    /// Tekst inhoud van de cel
    pub text: String,

    /// Rij index
    pub row: usize,

    /// Kolom index
    pub col: usize,

    /// Of de cel samengevoegd is (merged)
    pub merged: bool,
}

impl Default for TableCell {
    fn default() -> Self {
        Self {
            text: String::new(),
            row: 0,
            col: 0,
            merged: false,
        }
    }
}

/// Bounding box voor positionele informatie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    /// X coordinaat (links)
    pub x: f32,

    /// Y coordinaat (boven)
    pub y: f32,

    /// Breedte
    pub width: f32,

    /// Hoogte
    pub height: f32,
}

impl BoundingBox {
    /// Maak een nieuwe bounding box.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_chunk_builder() {
        let chunk = DocumentChunk::new("Test tekst".to_string())
            .with_metadata(0, Some(1))
            .with_offset(100, 9);

        assert_eq!(chunk.text, "Test tekst");
        assert_eq!(chunk.index, 0);
        assert_eq!(chunk.page, Some(1));
        assert_eq!(chunk.offset, Some(100));
    }

    #[test]
    fn test_table_builder() {
        let mut table = Table::new(2, 3).with_headers(vec![
            "Kolom 1".to_string(),
            "Kolom 2".to_string(),
            "Kolom 3".to_string(),
        ]);

        table.set_cell(0, 0, "Data 1".to_string());
        table.set_cell(0, 1, "Data 2".to_string());
        table.set_cell(0, 2, "Data 3".to_string());

        assert_eq!(table.headers.len(), 3);
        assert_eq!(table.cells[0][0].text, "Data 1");
    }
}
