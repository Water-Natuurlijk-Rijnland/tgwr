//! OCR (Optical Character Recognition) configuratie en backend.

use serde::{Deserialize, Serialize};

/// OCR backend opties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcrBackend {
    /// Geen OCR - alleen tekst extractie uit embedded PDF tekst
    None,

    /// Tesseract OCR
    Tesseract,

    /// EasyOCR
    EasyOcr,

    /// PaddleOCR
    PaddleOcr,
}

impl Default for OcrBackend {
    fn default() -> Self {
        Self::None
    }
}

impl OcrBackend {
    /// Converteer naar kreuzberg backend string.
    pub fn as_backend_str(&self) -> Option<&'static str> {
        match self {
            OcrBackend::Tesseract => Some("tesseract"),
            OcrBackend::EasyOcr => Some("easyocr"),
            OcrBackend::PaddleOcr => Some("paddleocr"),
            OcrBackend::None => None,
        }
    }
}

/// OCR configuratie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// Te gebruiken OCR backend
    pub backend: OcrBackend,

    /// Taal voor Tesseract (ISO 639-2 code, bijv. "eng", "nld")
    pub language: String,

    /// Of offset (positie) informatie bijgehouden moet worden
    pub include_offsets: bool,

    /// Pad naar Tesseract executable (optioneel, zoekt in PATH indien None)
    pub tesseract_path: Option<String>,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            backend: OcrBackend::None,
            language: "nld".to_string(),
            include_offsets: true,
            tesseract_path: None,
        }
    }
}

impl OcrConfig {
    /// Maak een nieuwe OCR configuratie.
    pub fn new(backend: OcrBackend, language: impl Into<String>) -> Self {
        Self {
            backend,
            language: language.into(),
            include_offsets: true,
            tesseract_path: None,
        }
    }

    /// Zet de taal voor OCR.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Zet of offsets bijgehouden moeten worden.
    pub fn with_offsets(mut self, include: bool) -> Self {
        self.include_offsets = include;
        self
    }

    /// Zet een custom Tesseract pad.
    pub fn with_tesseract_path(mut self, path: impl Into<String>) -> Self {
        self.tesseract_path = Some(path.into());
        self
    }

    /// Zet de OCR backend.
    pub fn with_backend(mut self, backend: OcrBackend) -> Self {
        self.backend = backend;
        self
    }

    /// Converteer naar kreuzberg OcrConfig indien nodig.
    pub fn as_kreuzberg_ocr_config(&self) -> Option<kreuzberg::OcrConfig> {
        match self.backend {
            OcrBackend::Tesseract => Some(kreuzberg::OcrConfig {
                backend: "tesseract".to_string(),
                language: self.language.clone(),
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
            }),
            OcrBackend::EasyOcr => Some(kreuzberg::OcrConfig {
                backend: "easyocr".to_string(),
                language: self.language.clone(),
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
            }),
            OcrBackend::PaddleOcr => Some(kreuzberg::OcrConfig {
                backend: "paddleocr".to_string(),
                language: self.language.clone(),
                tesseract_config: None,
                output_format: None,
                paddle_ocr_config: None,
                element_config: None,
            }),
            OcrBackend::None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_config_default() {
        let config = OcrConfig::default();
        assert_eq!(config.backend, OcrBackend::None);
        assert_eq!(config.language, "nld");
        assert!(config.include_offsets);
    }

    #[test]
    fn test_ocr_config_builder() {
        let config = OcrConfig::new(OcrBackend::Tesseract, "eng")
            .with_offsets(false)
            .with_tesseract_path("/usr/bin/tesseract");

        assert_eq!(config.backend, OcrBackend::Tesseract);
        assert_eq!(config.language, "eng");
        assert!(!config.include_offsets);
        assert_eq!(config.tesseract_path, Some("/usr/bin/tesseract".to_string()));
    }

    #[test]
    fn test_backend_str() {
        assert_eq!(OcrBackend::Tesseract.as_backend_str(), Some("tesseract"));
        assert_eq!(OcrBackend::None.as_backend_str(), None);
    }
}
