//! Visualisatie en grafiek generatie voor simulatieresultaten.
//!
//! Deze module biedt functionaliteit voor het genereren van grafieken en plots
//! van simulatieresultaten, met behulp van de plotters library.

use std::collections::HashMap;
use std::path::Path;

use crate::netwerk::{NetwerkSimulatieResultaat, PeilgebiedId};

/// Grafiek type.
#[derive(Debug, Clone, Copy)]
pub enum GrafiekType {
    /// Lijngrafiek voor tijdreeksen
    Lijn,
    /// Staafgrafiek
    Staaf,
    /// Verspreidingsplot
    Scatter,
}

/// Resolutie voor output afbeeldingen.
#[derive(Debug, Clone, Copy)]
pub struct Resolutie {
    pub breedte: u32,
    pub hoogte: u32,
}

impl Resolutie {
    /// Standaard HD resolutie (1920x1080)
    pub fn hd() -> Self {
        Self {
            breedte: 1920,
            hoogte: 1080,
        }
    }

    /// Standaard SD resolutie (1280x720)
    pub fn sd() -> Self {
        Self {
            breedte: 1280,
            hoogte: 720,
        }
    }

    /// 4K resolutie (3840x2160)
    pub fn vierk() -> Self {
        Self {
            breedte: 3840,
            hoogte: 2160,
        }
    }

    /// Custom resolutie
    pub fn nieuw(breedte: u32, hoogte: u32) -> Self {
        Self { breedte, hoogte }
    }
}

impl Default for Resolutie {
    fn default() -> Self {
        Self::hd()
    }
}

/// Kleurenschema voor grafieken.
#[derive(Debug, Clone, Copy)]
#[derive(Default)]
pub enum Kleurenschema {
    /// Standaard kleurenpalet
    #[default]
    Standaard,
    /// Kleurenblind-vriendelijk palet
    Kleurenblind,
    /// Grijstinten
    Grijs,
    /// Kleurrijk palet
    Kleurrijk,
}

impl Kleurenschema {
    /// Kleuren voor peilgebied (met wraparound voor veilige toegang)
    pub fn kleur_op_index(&self, index: usize) -> plotters::style::RGBColor {
        let kleuren = self.kleuren();
        kleuren[index % kleuren.len()]
    }

    /// Kleuren voor peilgebieden (max 8)
    pub fn kleuren(&self) -> Vec<plotters::style::RGBColor> {
        match self {
            Self::Standaard => vec
![
                plotters::style::RGBColor(0, 114, 178),   // Blauw
                plotters::style::RGBColor(213, 94, 0),    // Oranje
                plotters::style::RGBColor(0, 158, 115),   // Groen
                plotters::style::RGBColor(204, 121, 167), // Roze
                plotters::style::RGBColor(86, 180, 233),  // Lichtblauw
                plotters::style::RGBColor(230, 159, 0),   // Geel
                plotters::style::RGBColor(0, 0, 0),       // Zwart
                plotters::style::RGBColor(128, 128, 128), // Grijs
            ],
            Self::Kleurenblind => vec
![
                plotters::style::RGBColor(0, 114, 178),
                plotters::style::RGBColor(230, 159, 0),
                plotters::style::RGBColor(86, 180, 233),
                plotters::style::RGBColor(0, 158, 115),
                plotters::style::RGBColor(213, 94, 0),
                plotters::style::RGBColor(240, 228, 66),
                plotters::style::RGBColor(204, 121, 167),
                plotters::style::RGBColor(128, 128, 128),
            ],
            Self::Grijs => vec
![
                plotters::style::RGBColor(0, 0, 0),
                plotters::style::RGBColor(51, 51, 51),
                plotters::style::RGBColor(102, 102, 102),
                plotters::style::RGBColor(153, 153, 153),
                plotters::style::RGBColor(204, 204, 204),
                plotters::style::RGBColor(128, 128, 128),
                plotters::style::RGBColor(178, 178, 178),
                plotters::style::RGBColor(230, 230, 230),
            ],
            Self::Kleurrijk => vec
![
                plotters::style::RGBColor(255, 0, 0),
                plotters::style::RGBColor(0, 255, 0),
                plotters::style::RGBColor(0, 0, 255),
                plotters::style::RGBColor(255, 255, 0),
                plotters::style::RGBColor(255, 0, 255),
                plotters::style::RGBColor(0, 255, 255),
                plotters::style::RGBColor(128, 0, 128),
                plotters::style::RGBColor(255, 128, 0),
            ],
        }
    }
}


/// Fouttype voor visualisatie operaties.
#[derive(Debug, Clone, PartialEq)]
pub enum VisualisatieFout {
    /// Kon niet schrijven naar bestand
    SchrijvenMislukt { pad: String, reden: String },
    /// Ongeldige data voor visualisatie
    OngeldigeData { details: String },
    /// Geen data om te visualiseren
    GeenData,
}

impl std::fmt::Display for VisualisatieFout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchrijvenMislukt { pad, reden } => {
                write!(f, "Kon niet schrijven naar {}: {}", pad, reden)
            }
            Self::OngeldigeData { details } => {
                write!(f, "Ongeldige data: {}", details)
            }
            Self::GeenData => {
                write!(f, "Geen data om te visualiseren")
            }
        }
    }
}

impl std::error::Error for VisualisatieFout {}

impl From<std::io::Error> for VisualisatieFout {
    fn from(err: std::io::Error) -> Self {
        Self::SchrijvenMislukt {
            pad: "onbekend".to_string(),
            reden: err.to_string(),
        }
    }
}

/// Opties voor grafiek generatie.
#[derive(Debug, Clone)]
pub struct GrafiekOpties {
    /// Resolutie van de output afbeelding
    pub resolutie: Resolutie,
    /// Kleurenschema
    pub kleurenschema: Kleurenschema,
    /// Toon legenda
    pub toon_legenda: bool,
    /// Toon grid
    pub toon_grid: bool,
    /// Titel van de grafiek
    pub titel: Option<String>,
    /// X-as label
    pub x_label: Option<String>,
    /// Y-as label
    pub y_label: Option<String>,
}

impl Default for GrafiekOpties {
    fn default() -> Self {
        Self {
            resolutie: Resolutie::default(),
            kleurenschema: Kleurenschema::default(),
            toon_legenda: true,
            toon_grid: true,
            titel: None,
            x_label: Some("Tijd (minuten)".to_string()),
            y_label: None,
        }
    }
}

/// Waterstand grafiek generator.
pub struct WaterstandGrafiek {
    opties: GrafiekOpties,
}

impl WaterstandGrafiek {
    /// Maak een nieuwe waterstand grafiek generator.
    pub fn nieuw() -> Self {
        Self {
            opties: GrafiekOpties::default(),
        }
    }

    /// Maak met custom opties.
    pub fn met_opties(opties: GrafiekOpties) -> Self {
        Self { opties }
    }

    /// Genereer waterstand grafiek en sla op als PNG.
    pub fn naar_png<P: AsRef<Path>>(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        pad: P,
    ) -> Result<(), VisualisatieFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(VisualisatieFout::GeenData);
        }

        use plotters::prelude::*;

        let resolutie = self.opties.resolutie;
        let backend = BitMapBackend::new(pad.as_ref(), (resolutie.breedte, resolutie.hoogte))
            .into_drawing_area();

        backend.fill(&WHITE).map_err(|e| VisualisatieFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        let max_tijd = resultaat
            .tijdstappen
            .last()
            .map(|t| t.tijd)
            .unwrap_or(1.0);

        let mut chart = ChartBuilder::on(&backend)
            .margin(10u32)
            .caption(
                self.opties.titel.as_deref().unwrap_or("Waterstand Verloop"),
                ("sans-serif", 40u32),
            )
            .x_label_area_size(60u32)
            .y_label_area_size(80u32)
            .build_cartesian_2d(
                0f64..max_tijd,
                self.bepaal_y_bereik(resultaat),
            )
            .map_err(|e| VisualisatieFout::OngeldigeData {
                details: e.to_string(),
            })?;

        chart.configure_mesh().draw().map_err(|e| VisualisatieFout::OngeldigeData {
            details: e.to_string(),
        })?;

        // Verzamel alle peilgebied IDs
        let peilgebied_ids = if let Some(eerste) = resultaat.tijdstappen.first() {
            eerste.statussen.keys().cloned().collect::<Vec<_>>()
        } else {
            return Err(VisualisatieFout::GeenData);
        };

        for (i, id) in peilgebied_ids.iter().enumerate() {
            let kleur = self.opties.kleurenschema.kleur_op_index(i);

            let data: Vec<(f64, f64)> = resultaat
                .tijdstappen
                .iter()
                .filter_map(|stap| {
                    stap.statussen
                        .get(id)
                        .map(|status| (stap.tijd, status.waterstand))
                })
                .collect();

            chart
                .draw_series(LineSeries::new(data, kleur.stroke_width(3)))
                .map_err(|e| VisualisatieFout::OngeldigeData {
                    details: e.to_string(),
                })?
                .label(id.clone());
        }

        if self.opties.toon_legenda {
            chart
                .configure_series_labels()
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw()
                .map_err(|e| VisualisatieFout::OngeldigeData {
                    details: e.to_string(),
                })?;
        }

        backend.present().map_err(|e| VisualisatieFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        Ok(())
    }

    /// Genereer waterstand grafiek en sla op als SVG.
    pub fn naar_svg<P: AsRef<Path>>(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        pad: P,
    ) -> Result<(), VisualisatieFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(VisualisatieFout::GeenData);
        }

        use plotters::prelude::*;

        let resolutie = self.opties.resolutie;
        let backend = SVGBackend::new(pad.as_ref(), (resolutie.breedte, resolutie.hoogte))
            .into_drawing_area();

        backend.fill(&WHITE).map_err(|e| VisualisatieFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        let max_tijd = resultaat
            .tijdstappen
            .last()
            .map(|t| t.tijd)
            .unwrap_or(1.0);

        let mut chart = ChartBuilder::on(&backend)
            .margin(10u32)
            .caption(
                self.opties.titel.as_deref().unwrap_or("Waterstand Verloop"),
                ("sans-serif", 40u32),
            )
            .x_label_area_size(60u32)
            .y_label_area_size(80u32)
            .build_cartesian_2d(
                0f64..max_tijd,
                self.bepaal_y_bereik(resultaat),
            )
            .map_err(|e| VisualisatieFout::OngeldigeData {
                details: e.to_string(),
            })?;

        chart.configure_mesh().draw().map_err(|e| VisualisatieFout::OngeldigeData {
            details: e.to_string(),
        })?;

        let peilgebied_ids = if let Some(eerste) = resultaat.tijdstappen.first() {
            eerste.statussen.keys().cloned().collect::<Vec<_>>()
        } else {
            return Err(VisualisatieFout::GeenData);
        };

        for (i, id) in peilgebied_ids.iter().enumerate() {
            let kleur = self.opties.kleurenschema.kleur_op_index(i);

            let data: Vec<(f64, f64)> = resultaat
                .tijdstappen
                .iter()
                .filter_map(|stap| {
                    stap.statussen
                        .get(id)
                        .map(|status| (stap.tijd, status.waterstand))
                })
                .collect();

            chart
                .draw_series(LineSeries::new(data, kleur.stroke_width(3)))
                .map_err(|e| VisualisatieFout::OngeldigeData {
                    details: e.to_string(),
                })?
                .label(id.clone());
        }

        if self.opties.toon_legenda {
            chart
                .configure_series_labels()
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw()
                .map_err(|e| VisualisatieFout::OngeldigeData {
                    details: e.to_string(),
                })?;
        }

        backend.present().map_err(|e| VisualisatieFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        Ok(())
    }

    fn bepaal_y_bereik(&self, resultaat: &NetwerkSimulatieResultaat) -> std::ops::Range<f64> {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for stap in &resultaat.tijdstappen {
            for status in stap.statussen.values() {
                min = min.min(status.waterstand);
                max = max.max(status.waterstand);
            }
        }

        // Voeg marge toe
        let marge = (max - min) * 0.1;
        min - marge..max + marge
    }
}

impl Default for WaterstandGrafiek {
    fn default() -> Self {
        Self::nieuw()
    }
}

/// Regenintensiteit grafiek generator.
pub struct RegenGrafiek {
    opties: GrafiekOpties,
}

impl RegenGrafiek {
    /// Maak een nieuwe regen grafiek generator.
    pub fn nieuw() -> Self {
        Self {
            opties: GrafiekOpties::default(),
        }
    }

    /// Maak met custom opties.
    pub fn met_opties(opties: GrafiekOpties) -> Self {
        Self { opties }
    }

    /// Genereer regenintensiteit grafiek en sla op als PNG.
    pub fn naar_png<P: AsRef<Path>>(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        pad: P,
    ) -> Result<(), VisualisatieFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(VisualisatieFout::GeenData);
        }

        use plotters::prelude::*;

        let resolutie = self.opties.resolutie;
        let backend = BitMapBackend::new(pad.as_ref(), (resolutie.breedte, resolutie.hoogte))
            .into_drawing_area();

        backend.fill(&WHITE).map_err(|e| VisualisatieFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        let max_regen = self.bepaal_max_regen(resultaat);
        let max_tijd = resultaat
            .tijdstappen
            .last()
            .map(|t| t.tijd)
            .unwrap_or(1.0);

        let mut chart = ChartBuilder::on(&backend)
            .margin(10u32)
            .caption(
                self.opties.titel.as_deref().unwrap_or("Regenintensiteit"),
                ("sans-serif", 40u32),
            )
            .x_label_area_size(60u32)
            .y_label_area_size(80u32)
            .build_cartesian_2d(
                0f64..max_tijd,
                0f64..max_regen * 1.1,
            )
            .map_err(|e| VisualisatieFout::OngeldigeData {
                details: e.to_string(),
            })?;

        chart
            .configure_mesh()
            .y_desc("mm/uur")
            .draw()
            .map_err(|e| VisualisatieFout::OngeldigeData {
                details: e.to_string(),
            })?;

        let peilgebied_ids = if let Some(eerste) = resultaat.tijdstappen.first() {
            eerste.statussen.keys().cloned().collect::<Vec<_>>()
        } else {
            return Err(VisualisatieFout::GeenData);
        };

        for (i, id) in peilgebied_ids.iter().enumerate() {
            let kleur = self.opties.kleurenschema.kleur_op_index(i);

            let data: Vec<(f64, f64)> = resultaat
                .tijdstappen
                .iter()
                .filter_map(|stap| {
                    stap.statussen
                        .get(id)
                        .map(|status| (stap.tijd, status.regen_intensiteit))
                })
                .collect();

            chart
                .draw_series(LineSeries::new(data, kleur.stroke_width(2)))
                .map_err(|e| VisualisatieFout::OngeldigeData {
                    details: e.to_string(),
                })?
                .label(id.clone());
        }

        if self.opties.toon_legenda {
            chart
                .configure_series_labels()
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw()
                .map_err(|e| VisualisatieFout::OngeldigeData {
                    details: e.to_string(),
                })?;
        }

        backend.present().map_err(|e| VisualisatieFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        Ok(())
    }

    fn bepaal_max_regen(&self, resultaat: &NetwerkSimulatieResultaat) -> f64 {
        let mut max: f64 = 0.0;
        for stap in &resultaat.tijdstappen {
            for status in stap.statussen.values() {
                max = max.max(status.regen_intensiteit);
            }
        }
        max.max(1.0) // Minimaal 1 mm/uur als y-bereik
    }
}

impl Default for RegenGrafiek {
    fn default() -> Self {
        Self::nieuw()
    }
}

/// Pompcyclus grafiek generator.
pub struct PompGrafiek {
    opties: GrafiekOpties,
}

impl PompGrafiek {
    /// Maak een nieuwe pomp grafiek generator.
    pub fn nieuw() -> Self {
        Self {
            opties: GrafiekOpties::default(),
        }
    }

    /// Maak met custom opties.
    pub fn met_opties(opties: GrafiekOpties) -> Self {
        Self { opties }
    }

    /// Genereer pompcyclus grafiek en sla op als PNG.
    /// Toont wanneer pompen aan/uit staan als "stap" functie.
    pub fn naar_png<P: AsRef<Path>>(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        pad: P,
    ) -> Result<(), VisualisatieFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(VisualisatieFout::GeenData);
        }

        use plotters::prelude::*;

        let resolutie = self.opties.resolutie;
        let backend = BitMapBackend::new(pad.as_ref(), (resolutie.breedte, resolutie.hoogte))
            .into_drawing_area();

        backend.fill(&WHITE).map_err(|e| VisualisatieFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        let max_tijd = resultaat
            .tijdstappen
            .last()
            .map(|t| t.tijd)
            .unwrap_or(1.0);

        let mut chart = ChartBuilder::on(&backend)
            .margin(10u32)
            .caption(
                self.opties.titel.as_deref().unwrap_or("Pompcycli"),
                ("sans-serif", 40u32),
            )
            .x_label_area_size(60u32)
            .y_label_area_size(80u32)
            .build_cartesian_2d(
                0f64..max_tijd,
                -0.5f64..2.5,
            )
            .map_err(|e| VisualisatieFout::OngeldigeData {
                details: e.to_string(),
            })?;

        chart
            .configure_mesh()
            .y_labels(3)
            .y_label_formatter(&|v| match *v as i32 {
                0 => "UIT".to_string(),
                1 => "AAN".to_string(),
                _ => "".to_string(),
            })
            .draw()
            .map_err(|e| VisualisatieFout::OngeldigeData {
                details: e.to_string(),
            })?;

        let peilgebied_ids = if let Some(eerste) = resultaat.tijdstappen.first() {
            eerste.statussen.keys().cloned().collect::<Vec<_>>()
        } else {
            return Err(VisualisatieFout::GeenData);
        };

        // Offset elke peilgebied verticaal voor duidelijkheid
        for (i, id) in peilgebied_ids.iter().enumerate() {
            let kleur = self.opties.kleurenschema.kleur_op_index(i);
            let offset = i as f64 * 3.0;

            // Verdeel in segmenten (aan/uit perioden)
            let data = self.compacteer_pomp_data(resultaat, id, offset);

            chart
                .draw_series(data.into_iter().map(|(start, einde, aan)| {
                    let y = if aan { 1.0 + offset } else { 0.0 + offset };
                    Rectangle::new(
                        [(start, y - 0.4), (einde, y + 0.4)],
                        kleur.filled(),
                    )
                }))
                .map_err(|e| VisualisatieFout::OngeldigeData {
                    details: e.to_string(),
                })?
                .label(id.clone());
        }

        if self.opties.toon_legenda {
            chart
                .configure_series_labels()
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw()
                .map_err(|e| VisualisatieFout::OngeldigeData {
                    details: e.to_string(),
                })?;
        }

        backend.present().map_err(|e| VisualisatieFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        Ok(())
    }

    fn compacteer_pomp_data(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        id: &PeilgebiedId,
        _offset: f64,
    ) -> Vec<(f64, f64, bool)> {
        let mut segments = Vec::new();
        let mut current_start: Option<f64> = None;
        let mut current_state: Option<bool> = None;

        for stap in &resultaat.tijdstappen {
            if let Some(status) = stap.statussen.get(id) {
                let new_state = status.pomp_actief;

                match current_state {
                    None => {
                        current_start = Some(stap.tijd);
                        current_state = Some(new_state);
                    }
                    Some(state) if state != new_state => {
                        // Segment afsluiten
                        if let Some(start) = current_start {
                            segments.push((start, stap.tijd, state));
                        }
                        current_start = Some(stap.tijd);
                        current_state = Some(new_state);
                    }
                    _ => {}
                }
            }
        }

        // Laatste segment afsluiten
        if let (Some(start), Some(state), Some(laatste)) = (
            current_start,
            current_state,
            resultaat.tijdstappen.last(),
        ) {
            segments.push((start, laatste.tijd + 1.0, state));
        }

        segments
    }
}

impl Default for PompGrafiek {
    fn default() -> Self {
        Self::nieuw()
    }
}

/// Genereer alle standaard grafieken voor een simulatieresultaat.
pub fn genereer_alle_grafieken<P: AsRef<Path>>(
    resultaat: &NetwerkSimulatieResultaat,
    map_pad: P,
) -> Result<HashMap<String, String>, VisualisatieFout> {
    let mut gegenereerd = HashMap::new();

    let pad = map_pad.as_ref();

    // Waterstand grafiek
    let waterstand_pad = pad.join("waterstand.png");
    WaterstandGrafiek::nieuw().naar_png(resultaat, &waterstand_pad)?;
    gegenereerd.insert(
        "waterstand".to_string(),
        waterstand_pad.display().to_string(),
    );

    // Regen grafiek
    let regen_pad = pad.join("regen.png");
    RegenGrafiek::nieuw().naar_png(resultaat, &regen_pad)?;
    gegenereerd.insert("regen".to_string(), regen_pad.display().to_string());

    // Pomp grafiek
    let pomp_pad = pad.join("pompcycli.png");
    PompGrafiek::nieuw().naar_png(resultaat, &pomp_pad)?;
    gegenereerd.insert("pompcycli".to_string(), pomp_pad.display().to_string());

    Ok(gegenereerd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netwerk::*;

    #[test]
    fn test_resolutie_hd() {
        let res = Resolutie::hd();
        assert_eq!(res.breedte, 1920);
        assert_eq!(res.hoogte, 1080);
    }

    #[test]
    fn test_resolutie_sd() {
        let res = Resolutie::sd();
        assert_eq!(res.breedte, 1280);
        assert_eq!(res.hoogte, 720);
    }

    #[test]
    fn test_resolutie_custom() {
        let res = Resolutie::nieuw(800, 600);
        assert_eq!(res.breedte, 800);
        assert_eq!(res.hoogte, 600);
    }

    #[test]
    fn test_kleurenschema_standaard() {
        let schema = Kleurenschema::Standaard;
        let kleuren = schema.kleuren();
        assert_eq!(kleuren.len(), 8);
    }

    #[test]
    fn test_kleurenschema_kleurenblind() {
        let schema = Kleurenschema::Kleurenblind;
        let kleuren = schema.kleuren();
        assert_eq!(kleuren.len(), 8);
    }

    #[test]
    fn test_grafiek_opties_default() {
        let opties = GrafiekOpties::default();
        assert!(opties.toon_legenda);
        assert!(opties.toon_grid);
        assert_eq!(
            opties.x_label,
            Some("Tijd (minuten)".to_string())
        );
    }

    #[test]
    fn test_visualisatie_fout_display() {
        let fout = VisualisatieFout::GeenData;
        let display = format!("{}", fout);
        assert!(display.contains("Geen data"));
    }

    #[test]
    fn test_waterstand_grafiek_leeg_resultaat() {
        let resultaat = NetwerkSimulatieResultaat {
            tijdstappen: vec
![],
            totale_kosten: None,
        };

        let grafiek = WaterstandGrafiek::nieuw();
        let result = grafiek.naar_png(&resultaat, "/tmp/test.png");

        assert!(matches!(result, Err(VisualisatieFout::GeenData)));
    }

    #[test]
    fn test_regen_grafiek_leeg_resultaat() {
        let resultaat = NetwerkSimulatieResultaat {
            tijdstappen: vec
![],
            totale_kosten: None,
        };

        let grafiek = RegenGrafiek::nieuw();
        let result = grafiek.naar_png(&resultaat, "/tmp/test_regen.png");

        assert!(matches!(result, Err(VisualisatieFout::GeenData)));
    }

    #[test]
    fn test_genereer_alle_grafieken_leeg() {
        let resultaat = NetwerkSimulatieResultaat {
            tijdstappen: vec
![],
            totale_kosten: None,
        };

        let result = genereer_alle_grafieken(&resultaat, "/tmp");

        assert!(matches!(result, Err(VisualisatieFout::GeenData)));
    }
}
