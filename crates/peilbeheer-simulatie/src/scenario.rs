//! Scenario beheer voor netwerksimulaties.
//!
//! Deze module biedt functionaliteit voor het opslaan en laden van simulatiescenario's,
//! inclusief netwerktopologie, regengegevens en simulatieparameters.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::netwerk::{
    NetwerkSimulatieResultaat, NetwerkTopologie, PeilgebiedId,
};

/// Een compleet simulatiescenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Unieke identificatie
    pub id: String,
    /// Naam van het scenario
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naam: Option<String>,
    /// Beschrijving van het scenario
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beschrijving: Option<String>,
    /// Netwerktopologie
    pub topologie: NetwerkTopologie,
    /// Regenscenario per peilgebied
    #[serde(default)]
    pub regen_scenario: Regenscenario,
    /// Simulatieparameters
    #[serde(default)]
    pub parameters: SimulatieParameters,
    /// Metagegevens
    #[serde(default)]
    pub metadata: ScenarioMetadata,
}

/// Regenscenario definitie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regenscenario {
    /// Regenintensiteit per uur per peilgebied (mm/uur)
    #[serde(default)]
    pub regen_per_uur: HashMap<PeilgebiedId, Vec<f64>>,
    /// Type regenscenario
    #[serde(default)]
    pub scenario_type: RegenscenarioType,
}

/// Type regenscenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegenscenarioType {
    /// Historische data
    Historisch,
    /// Ontworpen gebeurtenis (design storm)
    Ontworpen,
    /// Synthetisch gegenereerd
    Synthetisch,
    /// Constante regenval
    Constant,
}

impl Default for RegenscenarioType {
    fn default() -> Self {
        Self::Synthetisch
    }
}

impl Default for Regenscenario {
    fn default() -> Self {
        Self {
            regen_per_uur: HashMap::new(),
            scenario_type: RegenscenarioType::default(),
        }
    }
}

/// Simulatieparameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatieParameters {
    /// Duur in uren
    #[serde(default = "default_duration")]
    pub duration_hours: usize,
    /// Tijdstap in minuten
    #[serde(default = "default_timestep")]
    pub timestep_minutes: f64,
    /// Uitstroom strategy type
    #[serde(default)]
    pub strategy_type: StrategyType,
}

fn default_duration() -> usize {
    24
}

fn default_timestep() -> f64 {
    1.0
}

impl Default for SimulatieParameters {
    fn default() -> Self {
        Self {
            duration_hours: default_duration(),
            timestep_minutes: default_timestep(),
            strategy_type: StrategyType::default(),
        }
    }
}

/// Type uitstroomstrategy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyType {
    /// Simpele strategy: pomp als waterstand > streefpeil
    Simpel,
    /// Gebalanceerde strategy: verdeel waterlast
    Gebalanceerd { balance_factor: f64 },
}

impl Default for StrategyType {
    fn default() -> Self {
        Self::Simpel
    }
}

/// Metagegevens voor een scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMetadata {
    /// Aanmaakdatum
    #[serde(default = "utc_now")]
    pub aangemaakt: DateTime<Utc>,
    /// Laatste wijziging
    #[serde(default = "utc_now")]
    pub gewijzigd: DateTime<Utc>,
    /// Auteur
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auteur: Option<String>,
    /// Versie
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versie: Option<String>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
}

fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

impl Default for ScenarioMetadata {
    fn default() -> Self {
        Self {
            aangemaakt: utc_now(),
            gewijzigd: utc_now(),
            auteur: None,
            versie: None,
            tags: Vec::new(),
        }
    }
}

/// Resultaat van een scenario inclusief simulatieoutput.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResultaat {
    /// Het scenario
    pub scenario: Scenario,
    /// Simulatieresultaat
    pub resultaat: NetwerkSimulatieResultaat,
    /// Tijdstip van uitvoering
    #[serde(default = "utc_now")]
    pub uitgevoerd: DateTime<Utc>,
}

/// Fouttype voor scenario operaties.
#[derive(Debug, Clone, PartialEq)]
pub enum ScenarioFout {
    /// Kon scenario niet laden
    LadenMislukt { pad: String, reden: String },
    /// Kon scenario niet opslaan
    OpslaanMislukt { pad: String, reden: String },
    /// Ongeldig scenario formaat
    OngeldigFormaat { details: String },
    /// Scenario niet gevonden
    NietGevonden { id: String },
}

impl std::fmt::Display for ScenarioFout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LadenMislukt { pad, reden } => {
                write!(f, "Kon scenario niet laden van {}: {}", pad, reden)
            }
            Self::OpslaanMislukt { pad, reden } => {
                write!(f, "Kon scenario niet opslaan naar {}: {}", pad, reden)
            }
            Self::OngeldigFormaat { details } => {
                write!(f, "Ongeldig scenario formaat: {}", details)
            }
            Self::NietGevonden { id } => {
                write!(f, "Scenario niet gevonden: {}", id)
            }
        }
    }
}

impl std::error::Error for ScenarioFout {}

impl From<io::Error> for ScenarioFout {
    fn from(err: io::Error) -> Self {
        Self::LadenMislukt {
            pad: "onbekend".to_string(),
            reden: err.to_string(),
        }
    }
}

impl Scenario {
    /// Maak een nieuw scenario.
    pub fn nieuw(id: String, topologie: NetwerkTopologie) -> Self {
        Self {
            id,
            naam: None,
            beschrijving: None,
            topologie,
            regen_scenario: Regenscenario::default(),
            parameters: SimulatieParameters::default(),
            metadata: ScenarioMetadata::default(),
        }
    }

    /// Stel de naam in.
    pub fn met_naam(mut self, naam: String) -> Self {
        self.naam = Some(naam);
        self
    }

    /// Stel de beschrijving in.
    pub fn met_beschrijving(mut self, beschrijving: String) -> Self {
        self.beschrijving = Some(beschrijving);
        self
    }

    /// Stel het regenscenario in.
    pub fn met_regen_scenario(mut self, regen: Regenscenario) -> Self {
        self.regen_scenario = regen;
        self
    }

    /// Stel de simulatieparameters in.
    pub fn met_parameters(mut self, params: SimulatieParameters) -> Self {
        self.parameters = params;
        self
    }

    /// Stel metagegevens in.
    pub fn met_metadata(mut self, metadata: ScenarioMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Voeg een tag toe.
    pub fn voeg_tag_toe(&mut self, tag: String) {
        self.metadata.tags.push(tag);
    }

    /// Valideer het scenario.
    pub fn valideer(&self) -> Result<(), ScenarioFout> {
        // Controleer dat topologie geldig is
        self.topologie
            .valideer()
            .map_err(|e| ScenarioFout::OngeldigFormaat {
                details: format!("Ongeldige topologie: {}", e),
            })?;

        // Controleer dat regenscenario data past bij topologie
        for (id, regen_data) in &self.regen_scenario.regen_per_uur {
            if !self.topologie.peilgebieden.contains_key(id) {
                return Err(ScenarioFout::OngeldigFormaat {
                    details: format!(
                        "Regengegevens voor onbekend peilgebied: {}",
                        id
                    ),
                });
            }
            if regen_data.len() > self.parameters.duration_hours {
                return Err(ScenarioFout::OngeldigFormaat {
                    details: format!(
                        "Regengegevens ({} uren) langer dan simulatie duur ({} uren)",
                        regen_data.len(),
                        self.parameters.duration_hours
                    ),
                });
            }
        }

        Ok(())
    }

    /// Sla scenario op naar JSON bestand.
    pub fn sla_op<P: AsRef<Path>>(&self, pad: P) -> Result<(), ScenarioFout> {
        self.valideer()?;

        let json = serde_json::to_string_pretty(self).map_err(|e| {
            ScenarioFout::OpslaanMislukt {
                pad: pad.as_ref().display().to_string(),
                reden: format!("Serialisatie fout: {}", e),
            }
        })?;

        fs::write(pad.as_ref(), json).map_err(|e| ScenarioFout::OpslaanMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        Ok(())
    }

    /// Laad scenario uit JSON bestand.
    pub fn laad<P: AsRef<Path>>(pad: P) -> Result<Self, ScenarioFout> {
        let inhoud = fs::read_to_string(pad.as_ref()).map_err(|e| {
            ScenarioFout::LadenMislukt {
                pad: pad.as_ref().display().to_string(),
                reden: e.to_string(),
            }
        })?;

        let scenario: Self = serde_json::from_str(&inhoud).map_err(|e| {
            ScenarioFout::LadenMislukt {
                pad: pad.as_ref().display().to_string(),
                reden: format!("Parse fout: {}", e),
            }
        })?;

        // Valideer het geladen scenario
        scenario.valideer()?;

        Ok(scenario)
    }

    /// Exporteer als JSON string.
    pub fn als_json(&self) -> Result<String, ScenarioFout> {
        self.valideer()?;
        serde_json::to_string_pretty(self).map_err(|e| ScenarioFout::OngeldigFormaat {
            details: format!("Serialisatie fout: {}", e),
        })
    }

    /// Importeer uit JSON string.
    pub fn uit_json(json: &str) -> Result<Self, ScenarioFout> {
        let scenario: Self = serde_json::from_str(json).map_err(|e| {
            ScenarioFout::OngeldigFormaat {
                details: format!("Parse fout: {}", e),
            }
        })?;

        scenario.valideer()?;

        Ok(scenario)
    }

    /// Maak een kopie met nieuwe ID.
    pub fn kopie_met_id(&self, nieuwe_id: String) -> Self {
        let mut kopie = self.clone();
        kopie.id = nieuwe_id.clone();
        kopie.metadata.gewijzigd = utc_now();
        kopie
    }

    /// Update wijzigingstijd.
    pub fn markeer_gewijzigd(&mut self) {
        self.metadata.gewijzigd = utc_now();
    }
}

impl ScenarioResultaat {
    /// Maak een nieuw scenarioresultaat.
    pub fn nieuw(scenario: Scenario, resultaat: NetwerkSimulatieResultaat) -> Self {
        Self {
            scenario,
            resultaat,
            uitgevoerd: utc_now(),
        }
    }

    /// Sla resultaat op naar JSON bestand.
    pub fn sla_op<P: AsRef<Path>>(&self, pad: P) -> Result<(), ScenarioFout> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            ScenarioFout::OpslaanMislukt {
                pad: pad.as_ref().display().to_string(),
                reden: format!("Serialisatie fout: {}", e),
            }
        })?;

        fs::write(pad.as_ref(), json).map_err(|e| ScenarioFout::OpslaanMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: e.to_string(),
        })?;

        Ok(())
    }

    /// Laad resultaat uit JSON bestand.
    pub fn laad<P: AsRef<Path>>(pad: P) -> Result<Self, ScenarioFout> {
        let inhoud = fs::read_to_string(pad.as_ref()).map_err(|e| {
            ScenarioFout::LadenMislukt {
                pad: pad.as_ref().display().to_string(),
                reden: e.to_string(),
            }
        })?;

        serde_json::from_str(&inhoud).map_err(|e| ScenarioFout::LadenMislukt {
            pad: pad.as_ref().display().to_string(),
            reden: format!("Parse fout: {}", e),
        })
    }
}

/// Bouwer voor scenario's met een fluent API.
pub struct ScenarioBouwer {
    id: String,
    naam: Option<String>,
    beschrijving: Option<String>,
    topologie: Option<NetwerkTopologie>,
    regen_scenario: Regenscenario,
    parameters: SimulatieParameters,
    auteur: Option<String>,
    versie: Option<String>,
    tags: Vec<String>,
}

impl ScenarioBouwer {
    /// Start een nieuwe bouwer.
    pub fn nieuw(id: String) -> Self {
        Self {
            id,
            naam: None,
            beschrijving: None,
            topologie: None,
            regen_scenario: Regenscenario::default(),
            parameters: SimulatieParameters::default(),
            auteur: None,
            versie: None,
            tags: Vec::new(),
        }
    }

    /// Stel de naam in.
    pub fn met_naam(mut self, naam: String) -> Self {
        self.naam = Some(naam);
        self
    }

    /// Stel de beschrijving in.
    pub fn met_beschrijving(mut self, beschrijving: String) -> Self {
        self.beschrijving = Some(beschrijving);
        self
    }

    /// Stel de topologie in.
    pub fn met_topologie(mut self, topologie: NetwerkTopologie) -> Self {
        self.topologie = Some(topologie);
        self
    }

    /// Voeg regengegevens toe voor een peilgebied.
    pub fn met_regen(
        mut self,
        peilgebied_id: String,
        regen_per_uur: Vec<f64>,
    ) -> Self {
        self.regen_scenario
            .regen_per_uur
            .insert(peilgebied_id, regen_per_uur);
        self
    }

    /// Stel het complete regenscenario in.
    pub fn met_regen_scenario(mut self, regen_scenario: Regenscenario) -> Self {
        self.regen_scenario = regen_scenario;
        self
    }

    /// Stel het regenscenario type in.
    pub fn met_regen_type(mut self, scenario_type: RegenscenarioType) -> Self {
        self.regen_scenario.scenario_type = scenario_type;
        self
    }

    /// Stel de simulatieduur in.
    pub fn met_duur(mut self, uren: usize) -> Self {
        self.parameters.duration_hours = uren;
        self
    }

    /// Stel de uitstoomstrategy in.
    pub fn met_strategy(mut self, strategy: StrategyType) -> Self {
        self.parameters.strategy_type = strategy;
        self
    }

    /// Stel de auteur in.
    pub fn met_auteur(mut self, auteur: String) -> Self {
        self.auteur = Some(auteur);
        self
    }

    /// Stel de versie in.
    pub fn met_versie(mut self, versie: String) -> Self {
        self.versie = Some(versie);
        self
    }

    /// Voeg een tag toe.
    pub fn voeg_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Bouw het scenario.
    pub fn bouw(self) -> Result<Scenario, ScenarioFout> {
        let topologie = self
            .topologie
            .ok_or_else(|| ScenarioFout::OngeldigFormaat {
                details: "Topologie is verplicht".to_string(),
            })?;

        let metadata = ScenarioMetadata {
            aangemaakt: utc_now(),
            gewijzigd: utc_now(),
            auteur: self.auteur,
            versie: self.versie,
            tags: self.tags,
        };

        let scenario = Scenario {
            id: self.id,
            naam: self.naam,
            beschrijving: self.beschrijving,
            topologie,
            regen_scenario: self.regen_scenario,
            parameters: self.parameters,
            metadata,
        };

        scenario.valideer()?;
        Ok(scenario)
    }
}

/// Helper functie om een constant regenscenario te maken.
pub fn constant_regen_scenario(
    peilgebied_ids: &[String],
    mm_per_uur: f64,
    duration_hours: usize,
) -> Regenscenario {
    let regen_per_uur = peilgebied_ids
        .iter()
        .map(|id| (id.clone(), vec![mm_per_uur; duration_hours]))
        .collect();

    Regenscenario {
        regen_per_uur,
        scenario_type: RegenscenarioType::Constant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netwerk::{PeilgebiedConfig, Verbinding};

    fn maak_test_topologie() -> NetwerkTopologie {
        let mut topologie = NetwerkTopologie::nieuw();

        topologie
            .voeg_peilgebied_toe(PeilgebiedConfig {
                id: "polder_a".to_string(),
                naam: Some("Polder A".to_string()),
                oppervlakte: 100_000.0,
                streefpeil: -0.60,
                marge: 0.20,
                maaiveld_niveau: 0.0,
                max_uitstroom_debiet: 0.5,
                verdamping: 0.0,
                infiltratie: 0.0,
            })
            .unwrap();

        topologie
            .voeg_peilgebied_toe(PeilgebiedConfig {
                id: "polder_b".to_string(),
                naam: Some("Polder B".to_string()),
                oppervlakte: 80_000.0,
                streefpeil: -0.60,
                marge: 0.20,
                maaiveld_niveau: 0.0,
                max_uitstroom_debiet: 0.4,
                verdamping: 0.0,
                infiltratie: 0.0,
            })
            .unwrap();

        // Voeg verbinding toe zodat netwerk verbonden is
        topologie
            .voeg_verbinding_toe(
                Verbinding::nieuw_gemaal(
                    "verbinding_ab".to_string(),
                    "polder_a".to_string(),
                    "polder_b".to_string(),
                    0.3,
                    2.0,
                )
                .unwrap(),
            )
            .unwrap();

        topologie
    }

    #[test]
    fn test_scenario_nieuw() {
        let topologie = maak_test_topologie();
        let scenario = Scenario::nieuw("test_scenario".to_string(), topologie);

        assert_eq!(scenario.id, "test_scenario");
        assert!(scenario.naam.is_none());
        assert!(scenario.beschrijving.is_none());
    }

    #[test]
    fn test_scenario_builder() {
        let topologie = maak_test_topologie();

        let scenario = ScenarioBouwer::nieuw("test".to_string())
            .met_naam("Test Scenario".to_string())
            .met_beschrijving("Een test scenario".to_string())
            .met_topologie(topologie.clone())
            .met_regen("polder_a".to_string(), vec![5.0; 24])
            .met_regen("polder_b".to_string(), vec![3.0; 24])
            .met_duur(24)
            .met_strategy(StrategyType::Simpel)
            .voeg_tag("test".to_string())
            .voeg_tag("demo".to_string())
            .bouw()
            .unwrap();

        assert_eq!(scenario.id, "test");
        assert_eq!(scenario.naam, Some("Test Scenario".to_string()));
        assert_eq!(scenario.beschrijving, Some("Een test scenario".to_string()));
        assert_eq!(scenario.parameters.duration_hours, 24);
        assert_eq!(scenario.metadata.tags.len(), 2);
        assert!(scenario.metadata.tags.contains(&"test".to_string()));
        assert!(scenario.metadata.tags.contains(&"demo".to_string()));
    }

    #[test]
    fn test_scenario_builder_zonder_topologie() {
        let result = ScenarioBouwer::nieuw("test".to_string()).bouw();

        assert!(matches!(result, Err(ScenarioFout::OngeldigFormaat { .. })));
    }

    #[test]
    fn test_scenario_valideer() {
        let topologie = maak_test_topologie();
        let mut scenario = Scenario::nieuw("test".to_string(), topologie);

        // Geldig scenario zonder regendata
        assert!(scenario.valideer().is_ok());

        // Ongeldig: regendata voor onbekend peilgebied
        scenario
            .regen_scenario
            .regen_per_uur
            .insert("onbekend".to_string(), vec![1.0; 10]);

        assert!(matches!(
            scenario.valideer(),
            Err(ScenarioFout::OngeldigFormaat { .. })
        ));
    }

    #[test]
    fn test_scenario_valideer_regen_lengte() {
        let topologie = maak_test_topologie();
        let mut scenario = Scenario::nieuw("test".to_string(), topologie);

        // Te lange regendata
        scenario
            .regen_scenario
            .regen_per_uur
            .insert("polder_a".to_string(), vec![1.0; 100]);

        assert!(matches!(
            scenario.valideer(),
            Err(ScenarioFout::OngeldigFormaat { .. })
        ));
    }

    #[test]
    fn test_scenario_json_serialisatie() {
        let topologie = maak_test_topologie();
        let scenario = Scenario::nieuw("test".to_string(), topologie)
            .met_naam("Test".to_string());

        let json = scenario.als_json().unwrap();
        let herladen = Scenario::uit_json(&json).unwrap();

        assert_eq!(herladen.id, scenario.id);
        assert_eq!(herladen.naam, scenario.naam);
    }

    #[test]
    fn test_scenario_kopie() {
        let topologie = maak_test_topologie();
        let scenario = Scenario::nieuw("origineel".to_string(), topologie.clone());

        let kopie = scenario.kopie_met_id("kopie".to_string());

        assert_eq!(kopie.id, "kopie");
        assert_ne!(kopie.id, "origineel");
    }

    #[test]
    fn test_constant_regen_scenario() {
        let ids = vec
!["a".to_string(), "b".to_string()];
        let regen = constant_regen_scenario(&ids, 5.0, 10);

        assert_eq!(regen.regen_per_uur.len(), 2);
        assert_eq!(
            regen.regen_per_uur.get("a").unwrap().len(),
            10
        );
        assert!(regen.regen_per_uur.get("a").unwrap().iter().all(|&x| x == 5.0));
    }

    #[test]
    fn test_regen_scenario_type_default() {
        let regen = Regenscenario::default();
        assert_eq!(regen.scenario_type, RegenscenarioType::Synthetisch);
    }

    #[test]
    fn test_scenario_parameters_default() {
        let params = SimulatieParameters::default();
        assert_eq!(params.duration_hours, 24);
        assert_eq!(params.timestep_minutes, 1.0);
        assert_eq!(params.strategy_type, StrategyType::Simpel);
    }

    #[test]
    fn test_scenario_metadata_default() {
        let metadata = ScenarioMetadata::default();
        assert!(metadata.auteur.is_none());
        assert!(metadata.versie.is_none());
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_strategy_type_default() {
        let strategy = StrategyType::default();
        assert_eq!(strategy, StrategyType::Simpel);
    }

    #[test]
    fn test_scenario_met_naam() {
        let topologie = maak_test_topologie();
        let scenario = Scenario::nieuw("test".to_string(), topologie)
            .met_naam("Mijn Scenario".to_string());

        assert_eq!(scenario.naam, Some("Mijn Scenario".to_string()));
    }

    #[test]
    fn test_scenario_met_beschrijving() {
        let topologie = maak_test_topologie();
        let scenario = Scenario::nieuw("test".to_string(), topologie)
            .met_beschrijving("Dit is een test".to_string());

        assert_eq!(scenario.beschrijving, Some("Dit is een test".to_string()));
    }

    #[test]
    fn test_scenario_tags_toevoegen() {
        let topologie = maak_test_topologie();
        let mut scenario = Scenario::nieuw("test".to_string(), topologie);

        scenario.voeg_tag_toe("test".to_string());
        scenario.voeg_tag_toe("demo".to_string());

        assert_eq!(scenario.metadata.tags.len(), 2);
    }

    #[test]
    fn test_scenario_fout_display() {
        let fout = ScenarioFout::NietGevonden {
            id: "test".to_string(),
        };
        let display = format!("{}", fout);
        assert!(display.contains("test"));
    }

    #[test]
    fn test_scenario_bouwer_met_constant_regen() {
        let topologie = maak_test_topologie();

        let regen = constant_regen_scenario(
            &["polder_a".to_string(), "polder_b".to_string()],
            10.0,
            6,
        );

        let scenario = ScenarioBouwer::nieuw("test".to_string())
            .met_topologie(topologie)
            .met_regen_scenario(regen.clone())
            .met_duur(6)
            .bouw()
            .unwrap();

        assert_eq!(scenario.regen_scenario.regen_per_uur.len(), 2);
        assert_eq!(
            scenario.regen_scenario.regen_per_uur["polder_a"].len(),
            6
        );
    }

    #[test]
    fn test_regen_scenario_type_variants() {
        assert_eq!(RegenscenarioType::Historisch, RegenscenarioType::Historisch);
        assert_eq!(RegenscenarioType::Ontworpen, RegenscenarioType::Ontworpen);
        assert_eq!(RegenscenarioType::Synthetisch, RegenscenarioType::Synthetisch);
        assert_eq!(RegenscenarioType::Constant, RegenscenarioType::Constant);
    }

    #[test]
    fn test_strategy_type_variants() {
        let simpel = StrategyType::Simpel;
        let gebalanceerd = StrategyType::Gebalanceerd {
            balance_factor: 0.5,
        };

        assert_eq!(simpel, StrategyType::Simpel);
        assert!(matches!(gebalanceerd, StrategyType::Gebalanceerd { .. }));

        if let StrategyType::Gebalanceerd { balance_factor } = gebalanceerd {
            assert_eq!(balance_factor, 0.5);
        }
    }

    #[test]
    fn test_scenario_serialisatie_roundtrip() {
        let topologie = maak_test_topologie();
        let original = Scenario::nieuw("test".to_string(), topologie)
            .met_naam("Test Scenario".to_string())
            .met_beschrijving("Test beschrijving".to_string());

        let json = original.als_json().unwrap();
        let loaded = Scenario::uit_json(&json).unwrap();

        assert_eq!(loaded.id, original.id);
        assert_eq!(loaded.naam, original.naam);
        assert_eq!(loaded.beschrijving, original.beschrijving);
        assert_eq!(loaded.topologie.peilgebieden.len(), original.topologie.peilgebieden.len());
    }
}
