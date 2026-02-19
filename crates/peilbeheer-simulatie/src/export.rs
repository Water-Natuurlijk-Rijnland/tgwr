//! Export functionaliteit voor simulatieresultaten.
//!
//! Deze module biedt functionaliteit voor het exporteren van simulatieresultaten
//! naar verschillende formaten (CSV, JSON) met flexibele opties.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use crate::netwerk::{NetwerkSimulatieResultaat, PeilgebiedId};

/// Export opties.
#[derive(Debug, Clone)]
pub struct ExportOpties {
    /// Include header rij in CSV
    pub csv_header: bool,
    /// Decimaal precision voor floating point waarden
    pub decimalen: usize,
    /// Scheidingsteken voor CSV (standaard komma)
    pub csv_scheidingsteken: char,
    /// Include metadata in JSON export
    pub json_metadata: bool,
}

impl Default for ExportOpties {
    fn default() -> Self {
        Self {
            csv_header: true,
            decimalen: 3,
            csv_scheidingsteken: ',',
            json_metadata: false,
        }
    }
}

/// Fouttype voor export operaties.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFout {
    /// Kon niet schrijven naar bestand
    SchrijvenMislukt { pad: String, reden: String },
    /// Ongeldig export formaat
    OngeldigFormaat { formaat: String },
    /// Geen data om te exporteren
    GeenData,
}

impl std::fmt::Display for ExportFout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchrijvenMislukt { pad, reden } => {
                write!(f, "Kon niet schrijven naar {}: {}", pad, reden)
            }
            Self::OngeldigFormaat { formaat } => {
                write!(f, "Ongeldig export formaat: {}", formaat)
            }
            Self::GeenData => {
                write!(f, "Geen data om te exporteren")
            }
        }
    }
}

impl std::error::Error for ExportFout {}

impl From<std::io::Error> for ExportFout {
    fn from(err: std::io::Error) -> Self {
        Self::SchrijvenMislukt {
            pad: "onbekend".to_string(),
            reden: err.to_string(),
        }
    }
}

/// CSV export van tijdstappen.
pub struct CsvExport {
    opties: ExportOpties,
}

impl CsvExport {
    /// Maak een nieuwe CSV export met standaard opties.
    pub fn nieuw() -> Self {
        Self {
            opties: ExportOpties::default(),
        }
    }

    /// Maak een nieuwe CSV export met custom opties.
    pub fn met_opties(opties: ExportOpties) -> Self {
        Self { opties }
    }

    /// Exporteer tijdstappen als CSV string.
    pub fn als_string(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
    ) -> Result<String, ExportFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(ExportFout::GeenData);
        }

        let mut output = String::new();

        // Header
        if self.opties.csv_header {
            let header = self.maak_header(resultaat);
            output.push_str(&header);
            output.push('\n');
        }

        // Data rijen
        for rij in self.maak_rijen(resultaat) {
            output.push_str(&rij);
            output.push('\n');
        }

        Ok(output)
    }

    /// Exporteer tijdstappen naar CSV bestand.
    pub fn naar_bestand<P: AsRef<Path>>(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        pad: P,
    ) -> Result<(), ExportFout> {
        let inhoud = self.als_string(resultaat)?;

        std::fs::write(pad.as_ref(), inhoud).map_err(|e| ExportFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        Ok(())
    }

    /// Exporteer per-peilgebied data naar afzonderlijke CSV bestanden.
    pub fn per_peilgebied<P: AsRef<Path>>(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        map_pad: P,
    ) -> Result<HashMap<PeilgebiedId, String>, ExportFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(ExportFout::GeenData);
        }

        let map_pad = map_pad.as_ref();

        // Verzamel alle unieke peilgebied IDs
        let peilgebied_ids: Vec<PeilgebiedId> = resultaat
            .tijdstappen
            .first()
            .map(|t| t.statussen.keys().cloned().collect())
            .unwrap_or_default();

        let mut gegenereerd = HashMap::new();

        for id in peilgebied_ids {
            let csv = self.per_peilgebied_string(resultaat, &id)?;

            let bestandsnaam = format!("{}.csv", id);
            let vol_pad = map_pad.join(&bestandsnaam);

            std::fs::write(&vol_pad, &csv).map_err(|e| ExportFout::SchrijvenMislukt {
                pad: vol_pad.display().to_string(),
                reden: e.to_string(),
            })?;

            gegenereerd.insert(id, bestandsnaam);
        }

        Ok(gegenereerd)
    }

    /// Exporteer verbindingstromen naar CSV.
    pub fn verbindingen_als_string(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
    ) -> Result<String, ExportFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(ExportFout::GeenData);
        }

        let mut output = String::new();

        // Header voor verbindingen
        if self.opties.csv_header {
            output.push_str(&format!(
                "tijd{}verbinding_id{}richting{}debiet{}benutting{}actief\n",
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken
            ));
        }

        // Data rijen
        for stap in &resultaat.tijdstappen {
            for stroom in &stap.stromen {
                let richting = match stroom.richting {
                    crate::netwerk::StroomRichting::Naar => "naar",
                    crate::netwerk::StroomRichting::Terug => "terug",
                };
                output.push_str(&format!(
                    "{:.1}{}{}{}{}{}{}{}{}{}{}\n",
                    stap.tijd,
                    self.opties.csv_scheidingsteken,
                    stroom.verbinding_id,
                    self.opties.csv_scheidingsteken,
                    richting,
                    self.opties.csv_scheidingsteken,
                    format_getal(stroom.debiet, self.opties.decimalen),
                    self.opties.csv_scheidingsteken,
                    format_getal(stroom.benutting, self.opties.decimalen),
                    self.opties.csv_scheidingsteken,
                    if stroom.actief { 1 } else { 0 }
                ));
            }
        }

        Ok(output)
    }

    fn maak_header(&self, resultaat: &NetwerkSimulatieResultaat) -> String {
        let eerste = &resultaat.tijdstappen[0];
        let peilgebied_ids: Vec<&PeilgebiedId> = eerste.statussen.keys().collect();

        let mut header = String::from("tijd");

        for id in &peilgebied_ids {
            header.push(self.opties.csv_scheidingsteken);
            header.push_str(id);
            header.push_str("_waterstand");

            header.push(self.opties.csv_scheidingsteken);
            header.push_str(id);
            header.push_str("_inkomend");

            header.push(self.opties.csv_scheidingsteken);
            header.push_str(id);
            header.push_str("_uitgaand");

            header.push(self.opties.csv_scheidingsteken);
            header.push_str(id);
            header.push_str("_uitstroom");

            header.push(self.opties.csv_scheidingsteken);
            header.push_str(id);
            header.push_str("_regen");

            header.push(self.opties.csv_scheidingsteken);
            header.push_str(id);
            header.push_str("_pomp");
        }

        header
    }

    fn maak_rijen(&self, resultaat: &NetwerkSimulatieResultaat) -> Vec<String> {
        resultaat
            .tijdstappen
            .iter()
            .map(|stap| {
                let mut rij = format!("{:.1}", stap.tijd);

                for (_id, status) in stap.statussen.iter() {
                    rij.push(self.opties.csv_scheidingsteken);
                    rij.push_str(&format_getal(
                        status.waterstand,
                        self.opties.decimalen,
                    ));

                    rij.push(self.opties.csv_scheidingsteken);
                    rij.push_str(&format_getal(
                        status.inkomend_debiet,
                        self.opties.decimalen,
                    ));

                    rij.push(self.opties.csv_scheidingsteken);
                    rij.push_str(&format_getal(
                        status.uitgaand_debiet,
                        self.opties.decimalen,
                    ));

                    rij.push(self.opties.csv_scheidingsteken);
                    rij.push_str(&format_getal(
                        status.uitstroom_debiet,
                        self.opties.decimalen,
                    ));

                    rij.push(self.opties.csv_scheidingsteken);
                    rij.push_str(&format_getal(
                        status.regen_intensiteit,
                        self.opties.decimalen,
                    ));

                    rij.push(self.opties.csv_scheidingsteken);
                    rij.push_str(if status.pomp_actief { "1" } else { "0" });
                }

                rij
            })
            .collect()
    }

    fn per_peilgebied_string(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        peilgebied_id: &str,
    ) -> Result<String, ExportFout> {
        let mut output = String::new();

        // Header
        if self.opties.csv_header {
            output.push_str(&format!(
                "tijd{}waterstand{}inkomend_debiet{}uitgaand_debiet{}uitstroom_debiet{}regen_intensiteit{}pomp_actief\n",
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken,
                self.opties.csv_scheidingsteken
            ));
        }

        // Data rijen
        for stap in &resultaat.tijdstappen {
            if let Some(status) = stap.statussen.get(peilgebied_id) {
                output.push_str(&format!(
                    "{:.1}{}{}{}{}{}{}{}{}{}{}{}{}\n",
                    stap.tijd,
                    self.opties.csv_scheidingsteken,
                    format_getal(status.waterstand, self.opties.decimalen),
                    self.opties.csv_scheidingsteken,
                    format_getal(status.inkomend_debiet, self.opties.decimalen),
                    self.opties.csv_scheidingsteken,
                    format_getal(status.uitgaand_debiet, self.opties.decimalen),
                    self.opties.csv_scheidingsteken,
                    format_getal(status.uitstroom_debiet, self.opties.decimalen),
                    self.opties.csv_scheidingsteken,
                    format_getal(status.regen_intensiteit, self.opties.decimalen),
                    self.opties.csv_scheidingsteken,
                    if status.pomp_actief { 1 } else { 0 }
                ));
            }
        }

        Ok(output)
    }
}

impl Default for CsvExport {
    fn default() -> Self {
        Self::nieuw()
    }
}

/// JSON export van tijdstappen.
pub struct JsonExport {
    #[allow(dead_code)]
    opties: ExportOpties,
}

impl JsonExport {
    /// Maak een nieuwe JSON export met standaard opties.
    pub fn nieuw() -> Self {
        Self {
            opties: ExportOpties::default(),
        }
    }

    /// Maak een nieuwe JSON export met custom opties.
    pub fn met_opties(opties: ExportOpties) -> Self {
        Self { opties }
    }

    /// Exporteer tijdstappen als JSON string (compact).
    pub fn als_string(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
    ) -> Result<String, ExportFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(ExportFout::GeenData);
        }

        serde_json::to_string(resultaat).map_err(|e| ExportFout::OngeldigFormaat {
            formaat: format!("JSON serialisatie fout: {}", e),
        })
    }

    /// Exporteer tijdstappen als pretty JSON string.
    pub fn als_pretty_string(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
    ) -> Result<String, ExportFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(ExportFout::GeenData);
        }

        serde_json::to_string_pretty(resultaat).map_err(|e| ExportFout::OngeldigFormaat {
            formaat: format!("JSON serialisatie fout: {}", e),
        })
    }

    /// Exporteer tijdstappen naar JSON bestand.
    pub fn naar_bestand<P: AsRef<Path>>(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
        pad: P,
    ) -> Result<(), ExportFout> {
        let inhoud = self.als_pretty_string(resultaat)?;

        std::fs::write(pad.as_ref(), inhoud).map_err(|e| ExportFout::SchrijvenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        Ok(())
    }

    /// Exporteer als gestructureerde data per peilgebied.
    pub fn per_peilgebied(
        &self,
        resultaat: &NetwerkSimulatieResultaat,
    ) -> Result<HashMap<PeilgebiedId, PeilgebiedExportData>, ExportFout> {
        if resultaat.tijdstappen.is_empty() {
            return Err(ExportFout::GeenData);
        }

        let mut data_per_peilgebied: HashMap<PeilgebiedId, PeilgebiedExportData> = HashMap::new();

        // Initialiseer voor elk peilgebied
        if let Some(eerste) = resultaat.tijdstappen.first() {
            for id in eerste.statussen.keys() {
                data_per_peilgebied.insert(
                    id.clone(),
                    PeilgebiedExportData {
                        peilgebied_id: id.clone(),
                        tijdstappen: Vec::new(),
                    },
                );
            }
        }

        // Vul data
        for stap in &resultaat.tijdstappen {
            for (id, status) in &stap.statussen {
                if let Some(data) = data_per_peilgebied.get_mut(id) {
                    data.tijdstappen.push(PeilgebiedTijdstapExport {
                        tijd: stap.tijd,
                        waterstand: status.waterstand,
                        inkomend_debiet: status.inkomend_debiet,
                        uitgaand_debiet: status.uitgaand_debiet,
                        uitstroom_debiet: status.uitstroom_debiet,
                        regen_intensiteit: status.regen_intensiteit,
                        pomp_actief: status.pomp_actief,
                    });
                }
            }
        }

        Ok(data_per_peilgebied)
    }
}

impl Default for JsonExport {
    fn default() -> Self {
        Self::nieuw()
    }
}

/// Geëxporteerde data voor één peilgebied.
#[derive(Debug, Clone, Serialize)]
pub struct PeilgebiedExportData {
    pub peilgebied_id: PeilgebiedId,
    pub tijdstappen: Vec<PeilgebiedTijdstapExport>,
}

/// Geëxporteerde tijdstap voor één peilgebied.
#[derive(Debug, Clone, Serialize)]
pub struct PeilgebiedTijdstapExport {
    pub tijd: f64,
    pub waterstand: f64,
    pub inkomend_debiet: f64,
    pub uitgaand_debiet: f64,
    pub uitstroom_debiet: f64,
    pub regen_intensiteit: f64,
    pub pomp_actief: bool,
}

/// Statistieken van een simulatieresultaat.
#[derive(Debug, Clone, Serialize)]
pub struct SimulatieStatistieken {
    pub aantal_tijdstappen: usize,
    pub totale_tijd: f64,
    pub peilgebieden: HashMap<PeilgebiedId, PeilgebiedStatistieken>,
}

/// Statistieken voor één peilgebied.
#[derive(Debug, Clone, Serialize)]
pub struct PeilgebiedStatistieken {
    pub min_waterstand: f64,
    pub max_waterstand: f64,
    pub gem_waterstand: f64,
    pub totale_uitstroom: f64,
    pub pomp_uren: f64,
    pub gem_regen: f64,
}

/// Bereken statistieken van een simulatieresultaat.
pub fn bereken_statistieken(
    resultaat: &NetwerkSimulatieResultaat,
) -> Result<SimulatieStatistieken, ExportFout> {
    if resultaat.tijdstappen.is_empty() {
        return Err(ExportFout::GeenData);
    }

    let laatste_tijd = resultaat
        .tijdstappen
        .last()
        .map(|t| t.tijd)
        .unwrap_or(0.0);

    let mut peilgebied_stats: HashMap<PeilgebiedId, PeilgebiedStatistieken> = HashMap::new();

    // Initialiseer accumulatoren
    let mut waterstanden: HashMap<PeilgebiedId, Vec<f64>> = HashMap::new();
    let mut uitstroom_totalen: HashMap<PeilgebiedId, f64> = HashMap::new();
    let mut pomp_minuten: HashMap<PeilgebiedId, f64> = HashMap::new();
    let mut regen_totalen: HashMap<PeilgebiedId, f64> = HashMap::new();

    for stap in &resultaat.tijdstappen {
        for (id, status) in &stap.statussen {
            waterstanden
                .entry(id.clone())
                .or_default()
                .push(status.waterstand);

            *uitstroom_totalen.entry(id.clone()).or_insert(0.0) += status.uitstroom_debiet;

            if status.pomp_actief {
                *pomp_minuten.entry(id.clone()).or_insert(0.0) += 1.0;
            }

            *regen_totalen
                .entry(id.clone())
                .or_insert(0.0) += status.regen_intensiteit;
        }
    }

    // Bereken statistieken per peilgebied
    for (id, ws) in waterstanden {
        let min = ws.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = ws.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let gem: f64 = ws.iter().sum::<f64>() / ws.len() as f64;

        let _totale_uitstroom = *uitstroom_totalen.get(&id).unwrap_or(&0.0);
        let pomp_min = *pomp_minuten.get(&id).unwrap_or(&0.0);
        let pomp_uren = pomp_min / 60.0;
        let gem_regen = *regen_totalen.get(&id).unwrap_or(&0.0) / ws.len() as f64;

        peilgebied_stats.insert(
            id.clone(),
            PeilgebiedStatistieken {
                min_waterstand: min,
                max_waterstand: max,
                gem_waterstand: gem,
                totale_uitstroom: _totale_uitstroom,
                pomp_uren,
                gem_regen,
            },
        );
    }

    Ok(SimulatieStatistieken {
        aantal_tijdstappen: resultaat.tijdstappen.len(),
        totale_tijd: laatste_tijd,
        peilgebieden: peilgebied_stats,
    })
}

/// Exporteer statistieken als JSON.
pub fn statistieken_als_json(
    resultaat: &NetwerkSimulatieResultaat,
) -> Result<String, ExportFout> {
    let stats = bereken_statistieken(resultaat)?;
    serde_json::to_string_pretty(&stats).map_err(|e| ExportFout::OngeldigFormaat {
        formaat: format!("JSON serialisatie fout: {}", e),
    })
}

/// Helper functie om getallen te formatteren met fixed decimalen.
fn format_getal(waarde: f64, decimalen: usize) -> String {
    format!("{:.1$}", waarde, decimalen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netwerk::*;

    fn maak_test_resultaat() -> NetwerkSimulatieResultaat {
        let statussen = {
            let mut map = HashMap::new();
            map.insert(
                "polder_a".to_string(),
                PeilgebiedStatus {
                    id: "polder_a".to_string(),
                    waterstand: -0.50,
                    inkomend_debiet: 0.1,
                    uitgaand_debiet: 0.05,
                    uitstroom_debiet: 0.2,
                    regen_intensiteit: 5.0,
                    pomp_actief: true,
                },
            );
            map.insert(
                "polder_b".to_string(),
                PeilgebiedStatus {
                    id: "polder_b".to_string(),
                    waterstand: -0.55,
                    inkomend_debiet: 0.05,
                    uitgaand_debiet: 0.0,
                    uitstroom_debiet: 0.1,
                    regen_intensiteit: 5.0,
                    pomp_actief: true,
                },
            );
            map
        };

        let stromen = vec
![VerbindingStroom {
            verbinding_id: "v1".to_string(),
            debiet: 0.05,
            richting: StroomRichting::Naar,
            benutting: 0.167,
            actief: true,
        }];

        let tijdstappen = vec
![
            NetwerkTijdstap {
                tijd: 1.0,
                statussen: statussen.clone(),
                stromen: stromen.clone(),
            },
            NetwerkTijdstap {
                tijd: 2.0,
                statussen,
                stromen,
            },
        ];

        NetwerkSimulatieResultaat {
            tijdstappen,
            totale_kosten: None,
        }
    }

    #[test]
    fn test_csv_export_header() {
        let resultaat = maak_test_resultaat();
        let export = CsvExport::nieuw();
        let csv = export.als_string(&resultaat).unwrap();

        assert!(csv.starts_with("tijd"));
        assert!(csv.contains("polder_a_waterstand"));
        assert!(csv.contains("polder_b_waterstand"));
    }

    #[test]
    fn test_csv_export_zonder_header() {
        let resultaat = maak_test_resultaat();
        let opties = ExportOpties {
            csv_header: false,
            ..Default::default()
        };
        let export = CsvExport::met_opties(opties);
        let csv = export.als_string(&resultaat).unwrap();

        assert!(!csv.starts_with("tijd"));
        assert!(csv.starts_with("1.0"));
    }

    #[test]
    fn test_csv_export_aantal_rijen() {
        let resultaat = maak_test_resultaat();
        let export = CsvExport::nieuw();
        let csv = export.als_string(&resultaat).unwrap();

        let regels: Vec<&str> = csv.lines().collect();
        // 1 header + 2 data rijen
        assert_eq!(regels.len(), 3);
    }

    #[test]
    fn test_csv_export_leeg_resultaat() {
        let resultaat = NetwerkSimulatieResultaat {
            tijdstappen: vec![],
            totale_kosten: None,
        };

        let export = CsvExport::nieuw();
        let result = export.als_string(&resultaat);

        assert!(matches!(result, Err(ExportFout::GeenData)));
    }

    #[test]
    fn test_csv_export_verbindingen() {
        let resultaat = maak_test_resultaat();
        let export = CsvExport::nieuw();
        let csv = export.verbindingen_als_string(&resultaat).unwrap();

        assert!(csv.contains("verbinding_id"));
        assert!(csv.contains("v1"));
        assert!(csv.contains("naar"));
    }

    #[test]
    fn test_csv_export_per_peilgebied() {
        let resultaat = maak_test_resultaat();
        let export = CsvExport::nieuw();
        let csv = export.per_peilgebied_string(&resultaat, "polder_a").unwrap();

        assert!(csv.contains("waterstand"));
        assert!(csv.contains("-0.50"));
    }

    #[test]
    fn test_json_export() {
        let resultaat = maak_test_resultaat();
        let export = JsonExport::nieuw();
        let json = export.als_string(&resultaat).unwrap();

        // Moet geldige JSON zijn
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_json_export_pretty() {
        let resultaat = maak_test_resultaat();
        let export = JsonExport::nieuw();
        let json = export.als_pretty_string(&resultaat).unwrap();

        // Pretty JSON heeft newlines
        assert!(json.contains('\n'));
        assert!(json.contains("tijdstappen"));
    }

    #[test]
    fn test_json_export_leeg_resultaat() {
        let resultaat = NetwerkSimulatieResultaat {
            tijdstappen: vec![],
            totale_kosten: None,
        };

        let export = JsonExport::nieuw();
        let result = export.als_string(&resultaat);

        assert!(matches!(result, Err(ExportFout::GeenData)));
    }

    #[test]
    fn test_json_export_per_peilgebied() {
        let resultaat = maak_test_resultaat();
        let export = JsonExport::nieuw();
        let data = export.per_peilgebied(&resultaat).unwrap();

        assert!(data.contains_key("polder_a"));
        assert!(data.contains_key("polder_b"));

        let polder_a = &data["polder_a"];
        assert_eq!(polder_a.peilgebied_id, "polder_a");
        assert_eq!(polder_a.tijdstappen.len(), 2);
    }

    #[test]
    fn test_statistieken_berekenen() {
        let resultaat = maak_test_resultaat();
        let stats = bereken_statistieken(&resultaat).unwrap();

        assert_eq!(stats.aantal_tijdstappen, 2);
        assert_eq!(stats.totale_tijd, 2.0);
        assert!(stats.peilgebieden.contains_key("polder_a"));
        assert!(stats.peilgebieden.contains_key("polder_b"));

        let polder_a = &stats.peilgebieden["polder_a"];
        assert_eq!(polder_a.min_waterstand, -0.50);
        assert_eq!(polder_a.max_waterstand, -0.50);
        assert_eq!(polder_a.pomp_uren, 2.0 / 60.0);
    }

    #[test]
    fn test_statistieken_json() {
        let resultaat = maak_test_resultaat();
        let json = statistieken_als_json(&resultaat).unwrap();

        assert!(json.contains("aantal_tijdstappen"));
        assert!(json.contains("peilgebieden"));
        assert!(json.contains("min_waterstand"));
    }

    #[test]
    fn test_statistieken_leeg_resultaat() {
        let resultaat = NetwerkSimulatieResultaat {
            tijdstappen: vec![],
            totale_kosten: None,
        };

        let result = bereken_statistieken(&resultaat);

        assert!(matches!(result, Err(ExportFout::GeenData)));
    }

    #[test]
    fn test_format_getal() {
        assert_eq!(format_getal(1.23456, 2), "1.23");
        assert_eq!(format_getal(1.0, 3), "1.000");
        assert_eq!(format_getal(-0.5, 1), "-0.5");
    }

    #[test]
    fn test_export_opties_default() {
        let opties = ExportOpties::default();
        assert!(opties.csv_header);
        assert_eq!(opties.decimalen, 3);
        assert_eq!(opties.csv_scheidingsteken, ',');
    }

    #[test]
    fn test_csv_export_custom_scheidingsteken() {
        let resultaat = maak_test_resultaat();
        let opties = ExportOpties {
            csv_scheidingsteken: ';',
            ..Default::default()
        };
        let export = CsvExport::met_opties(opties);
        let csv = export.als_string(&resultaat).unwrap();

        assert!(csv.contains(';'));
        // Komma's mogen niet als scheidingsteken voorkomen
        let regels: Vec<&str> = csv.lines().collect();
        let eerste = regels[0];
        let delen: Vec<&str> = eerste.split(';').collect();
        assert!(delen.len() > 2);
    }

    #[test]
    fn test_export_fout_display() {
        let fout = ExportFout::GeenData;
        let display = format!("{}", fout);
        assert!(display.contains("Geen data"));
    }

    #[test]
    fn test_csv_export_decimalen() {
        let resultaat = maak_test_resultaat();
        let opties = ExportOpties {
            decimalen: 3,
            ..Default::default()
        };
        let export = CsvExport::met_opties(opties);
        let csv = export.als_string(&resultaat).unwrap();

        // 3 decimalen voor -0.50 -> -0.500
        assert!(csv.contains("-0.500"));
    }

    #[test]
    fn test_peilgebied_export_data_serialize() {
        let data = PeilgebiedExportData {
            peilgebied_id: "test".to_string(),
            tijdstappen: vec![PeilgebiedTijdstapExport {
                tijd: 1.0,
                waterstand: -0.5,
                inkomend_debiet: 0.1,
                uitgaand_debiet: 0.05,
                uitstroom_debiet: 0.2,
                regen_intensiteit: 5.0,
                pomp_actief: true,
            }],
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("waterstand"));
    }

    #[test]
    fn test_csv_export_peilgebied_statistics() {
        let resultaat = maak_test_resultaat();
        let stats = bereken_statistieken(&resultaat).unwrap();

        let polder_a = &stats.peilgebieden["polder_a"];
        assert_eq!(polder_a.gem_waterstand, -0.50);
        assert_eq!(polder_a.totale_uitstroom, 0.4); // 0.2 per stap * 2 stappen
    }
}
