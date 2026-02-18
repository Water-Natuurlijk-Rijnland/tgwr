//! Multi-peilgebied netwerksimulatie.
//!
//! Module voor het simuleren van waterstromen tussen verbonden peilgebieden.
//! Ondersteunt verschillende connectietypen (pompen, duikers, overstorten),
//! gecoördineerde regeling, en netwerktopologie.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::waterbalans::calculate_water_balance;

/// Unieke identificatie van een peilgebied in het netwerk.
pub type PeilgebiedId = String;

/// Unieke identificatie van een verbinding.
pub type VerbindingId = String;

/// Fouttype voor netwerksimulaties.
#[derive(Debug, Clone, PartialEq)]
pub enum NetwerkFout {
    /// Peilgebied niet gevonden
    PeilgebiedNietGevonden { id: PeilgebiedId },
    /// Verbinding niet gevonden
    VerbindingNietGevonden { id: VerbindingId },
    /// Verbinding bestaat al
    VerbindingBestaatAl { id: VerbindingId },
    /// Cyclische verbinding gedetecteerd
    CyclischeVerbinding { van: PeilgebiedId, naar: PeilgebiedId },
    /// Ongeldige capaciteit
    OngeldigeCapaciteit { debiet: f64 },
    /// Ongeldige verbinding (van == naar)
    OngeldigeVerbinding { id: PeilgebiedId },
    /// Netwerk is niet verbonden (geïsoleerde componenten)
    NietVerbonden,
    /// Constraint schending bij simulatie
    ConstraintSchending {
        peilgebied: PeilgebiedId,
        waterstand: f64,
        min: f64,
        max: f64,
    },
}

impl fmt::Display for NetwerkFout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PeilgebiedNietGevonden { id } => {
                write!(f, "Peilgebied niet gevonden: {}", id)
            }
            Self::VerbindingNietGevonden { id } => {
                write!(f, "Verbinding niet gevonden: {}", id)
            }
            Self::VerbindingBestaatAl { id } => {
                write!(f, "Verbinding bestaat al: {}", id)
            }
            Self::CyclischeVerbinding { van, naar } => {
                write!(f, "Cyclische verbinding: {} -> {}", van, naar)
            }
            Self::OngeldigeCapaciteit { debiet } => {
                write!(f, "Ongeldige capaciteit: {} m³/s (moet >= 0)", debiet)
            }
            Self::OngeldigeVerbinding { id } => {
                write!(f, "Ongeldige verbinding: van en naar zijn beide {}", id)
            }
            Self::NietVerbonden => {
                write!(f, "Netwerk is niet volledig verbonden")
            }
            Self::ConstraintSchending {
                peilgebied,
                waterstand,
                min,
                max,
            } => {
                write!(
                    f,
                    "Constraint schending in {}: waterstand={:.2} m, bereik=[{:.2}, {:.2}]",
                    peilgebied, waterstand, min, max
                )
            }
        }
    }
}

impl std::error::Error for NetwerkFout {}

/// Type verbinding tussen peilgebieden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerbindingType {
    /// Gemaal/duiker: actieve watertransport met capaciteitsrestrictie
    Gemaal,
    /// Overstort: passieve stroming bij hoogwater boven drempel
    Overstort,
    /// Keerklep/terugslagklep: eenrichtingverkeer alleen
    Keerklep,
    /// Open verbinding: vrije stroming beide richtingen
    OpenVerbinding,
}

impl VerbindingType {
    /// Of dit verbindingstype actieve regeling ondersteunt.
    pub fn is_actief(&self) -> bool {
        matches!(self, Self::Gemaal)
    }

    /// Of dit verbindingstype passieve stroming toestaat.
    pub fn is_passief(&self) -> bool {
        matches!(self, Self::Overstort | Self::OpenVerbinding)
    }

    /// Of dit verbindingstype eenrichtingverkeer is.
    pub fn is_eenrichting(&self) -> bool {
        matches!(self, Self::Gemaal | Self::Overstort | Self::Keerklep)
    }
}

/// Verbinding tussen twee peilgebieden.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verbinding {
    /// Unieke identificatie
    pub id: VerbindingId,
    /// Type verbinding
    pub verbinding_type: VerbindingType,
    /// Van peilgebied
    pub van_id: PeilgebiedId,
    /// Naar peilgebied
    pub naar_id: PeilgebiedId,
    /// Maximale debiet in m³/s
    pub capaciteit: f64,
    /// Drempelpeil in m NAP voor overstort (alleen voor Overstort type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overstort_drempel: Option<f64>,
    /// Hoogteverschil voor pompvermogenberekening (m)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opvoerhoogte: Option<f64>,
    /// Efficiëntie van pompinstallatie (0-1)
    #[serde(default = "default_efficiency")]
    pub efficiency: f64,
    /// Huidige stroomrichting (Some voor actieve regeling)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroomrichting: Option<StroomRichting>,
}

fn default_efficiency() -> f64 {
    0.70
}

impl Verbinding {
    /// Maak een nieuwe gemaalverbinding.
    pub fn nieuw_gemaal(
        id: VerbindingId,
        van_id: PeilgebiedId,
        naar_id: PeilgebiedId,
        capaciteit: f64,
        opvoerhoogte: f64,
    ) -> Result<Self, NetwerkFout> {
        if van_id == naar_id {
            return Err(NetwerkFout::OngeldigeVerbinding { id: van_id });
        }
        if capaciteit < 0.0 {
            return Err(NetwerkFout::OngeldigeCapaciteit { debiet: capaciteit });
        }

        Ok(Self {
            id,
            verbinding_type: VerbindingType::Gemaal,
            van_id,
            naar_id,
            capaciteit,
            overstort_drempel: None,
            opvoerhoogte: Some(opvoerhoogte),
            efficiency: default_efficiency(),
            stroomrichting: Some(StroomRichting::Naar),
        })
    }

    /// Maak een nieuwe overstortverbinding.
    pub fn nieuw_overstort(
        id: VerbindingId,
        van_id: PeilgebiedId,
        naar_id: PeilgebiedId,
        capaciteit: f64,
        drempel: f64,
    ) -> Result<Self, NetwerkFout> {
        if van_id == naar_id {
            return Err(NetwerkFout::OngeldigeVerbinding { id: van_id });
        }
        if capaciteit < 0.0 {
            return Err(NetwerkFout::OngeldigeCapaciteit { debiet: capaciteit });
        }

        Ok(Self {
            id,
            verbinding_type: VerbindingType::Overstort,
            van_id,
            naar_id,
            capaciteit,
            overstort_drempel: Some(drempel),
            opvoerhoogte: None,
            efficiency: default_efficiency(),
            stroomrichting: None,
        })
    }

    /// Maak een nieuwe keerklepverbinding.
    pub fn nieuw_keerklep(
        id: VerbindingId,
        van_id: PeilgebiedId,
        naar_id: PeilgebiedId,
        capaciteit: f64,
    ) -> Result<Self, NetwerkFout> {
        if van_id == naar_id {
            return Err(NetwerkFout::OngeldigeVerbinding { id: van_id });
        }
        if capaciteit < 0.0 {
            return Err(NetwerkFout::OngeldigeCapaciteit { debiet: capaciteit });
        }

        Ok(Self {
            id,
            verbinding_type: VerbindingType::Keerklep,
            van_id,
            naar_id,
            capaciteit,
            overstort_drempel: None,
            opvoerhoogte: None,
            efficiency: default_efficiency(),
            stroomrichting: Some(StroomRichting::Naar),
        })
    }

    /// Bereken pompvermogen in kW.
    pub fn pompvermogen_kw(&self, debiet_m3s: f64) -> Option<f64> {
        if !self.verbinding_type.is_actief() {
            return None;
        }

        let opvoerhoogte = self.opvoerhoogte?;
        let eff = if self.efficiency > 0.0 {
            self.efficiency
        } else {
            0.70
        };

        // P = ρ × g × Q × H / η
        // ρ = 1000 kg/m³, g = 9.81 m/s²
        Some(1000.0 * 9.81 * debiet_m3s * opvoerhoogte / eff / 1000.0)
    }
}

/// Stroomrichting voor een verbinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StroomRichting {
    /// Van van_id naar naar_id
    Naar,
    /// Van naar_id naar van_id (terugstroom)
    Terug,
}

/// Configuratie voor één peilgebied in het netwerk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeilgebiedConfig {
    /// Unieke identificatie
    pub id: PeilgebiedId,
    /// Naam van het peilgebied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naam: Option<String>,
    /// Oppervlakte in m²
    pub oppervlakte: f64,
    /// Streefpeil in m NAP
    pub streefpeil: f64,
    /// Toegestane marge rondom streefpeil (m)
    #[serde(default = "default_marge")]
    pub marge: f64,
    /// Maaiveld niveau in m NAP
    #[serde(default)]
    pub maaiveld_niveau: f64,
    /// Maximale pomp capaciteit naar buiten (m³/s)
    #[serde(default)]
    pub max_uitstroom_debiet: f64,
    /// Verdamping in mm/uur
    #[serde(default)]
    pub verdamping: f64,
    /// Infiltratie in mm/uur
    #[serde(default)]
    pub infiltratie: f64,
}

fn default_marge() -> f64 {
    0.20
}

impl PeilgebiedConfig {
    /// Bereken minimaal toelaatbaar peil.
    pub fn min_peil(&self) -> f64 {
        self.streefpeil - self.marge
    }

    /// Bereken maximaal toelaatbaar peil.
    pub fn max_peil(&self) -> f64 {
        self.streefpeil + self.marge
    }

    /// Controleer of waterstand binnen toegestane bandbreedte valt.
    pub fn is_waterstand_geldig(&self, waterstand: f64) -> bool {
        waterstand >= self.min_peil() && waterstand <= self.max_peil()
    }
}

/// Status van één peilgebied op een tijdstip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeilgebiedStatus {
    /// Peilgebied ID
    pub id: PeilgebiedId,
    /// Huidige waterstand in m NAP
    pub waterstand: f64,
    /// Actuele inkomende stroom van verbindingen (m³/s)
    pub inkomend_debiet: f64,
    /// Actuele uitgaande stroom naar verbindingen (m³/s)
    pub uitgaand_debiet: f64,
    /// Uitstroom debiet naar boezem/externe watergang (m³/s)
    pub uitstroom_debiet: f64,
    /// Regenintensiteit (mm/uur)
    pub regen_intensiteit: f64,
    /// Is de uitstroompomp actief?
    pub pomp_actief: bool,
}

/// Resultaat van waterstroomberekening over één verbinding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerbindingStroom {
    /// Verbinding ID
    pub verbinding_id: VerbindingId,
    /// Berekend debiet (m³/s), positief = van->naar
    pub debiet: f64,
    /// Stroomrichting
    pub richting: StroomRichting,
    /// Capaciteitsbenutting (0-1)
    pub benutting: f64,
    /// Is de verbinding actief (debiet > 0)?
    pub actief: bool,
}

/// Netwerktopologie: peilgebieden en hun verbindingen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetwerkTopologie {
    /// Alle peilgebieden
    pub peilgebieden: HashMap<PeilgebiedId, PeilgebiedConfig>,
    /// Alle verbindingen
    pub verbindingen: HashMap<VerbindingId, Verbinding>,
}

impl NetwerkTopologie {
    /// Maak een nieuwe lege topologie.
    pub fn nieuw() -> Self {
        Self {
            peilgebieden: HashMap::new(),
            verbindingen: HashMap::new(),
        }
    }

    /// Voeg een peilgebied toe.
    pub fn voeg_peilgebied_toe(&mut self, config: PeilgebiedConfig) -> Result<(), NetwerkFout> {
        if config.oppervlakte <= 0.0 {
            return Err(NetwerkFout::OngeldigeCapaciteit {
                debiet: config.oppervlakte,
            });
        }

        self.peilgebieden.insert(config.id.clone(), config);
        Ok(())
    }

    /// Voeg een verbinding toe.
    pub fn voeg_verbinding_toe(&mut self, verbinding: Verbinding) -> Result<(), NetwerkFout> {
        // Controleer dat beide peilgebieden bestaan
        if !self.peilgebieden.contains_key(&verbinding.van_id) {
            return Err(NetwerkFout::PeilgebiedNietGevonden {
                id: verbinding.van_id,
            });
        }
        if !self.peilgebieden.contains_key(&verbinding.naar_id) {
            return Err(NetwerkFout::PeilgebiedNietGevonden {
                id: verbinding.naar_id,
            });
        }

        // Controleer op cyclische verbindingen (A->B en B->A)
        if self.bestaat_verbinding_tussen(&verbinding.naar_id, &verbinding.van_id) {
            return Err(NetwerkFout::CyclischeVerbinding {
                van: verbinding.van_id.clone(),
                naar: verbinding.naar_id.clone(),
            });
        }

        self.verbindingen
            .insert(verbinding.id.clone(), verbinding);
        Ok(())
    }

    /// Controleer of er een verbinding bestaat tussen twee peilgebieden.
    pub fn bestaat_verbinding_tussen(
        &self,
        van_id: &str,
        naar_id: &str,
    ) -> bool {
        self.verbindingen
            .values()
            .any(|v| v.van_id == van_id && v.naar_id == naar_id)
    }

    /// Haal alle verbindingen op die vanuit een peilgebied vertrekken.
    pub fn verbindingen_vanuit(&self, peilgebied_id: &str) -> Vec<&Verbinding> {
        self.verbindingen
            .values()
            .filter(|v| v.van_id == peilgebied_id)
            .collect()
    }

    /// Haal alle verbindingen op die naar een peilgebied leiden.
    pub fn verbindingen_naar(&self, peilgebied_id: &str) -> Vec<&Verbinding> {
        self.verbindingen
            .values()
            .filter(|v| v.naar_id == peilgebied_id)
            .collect()
    }

    /// Controleer of het netwerk volledig verbonden is (geen geïsoleerde componenten).
    ///
    /// Een netwerk is verbonden als er een pad is tussen elke twee peilgebieden,
    /// ongeacht de stroomrichting van de verbindingen (de graf is undirected voor
    // connectivity checking).
    pub fn is_verbonden(&self) -> bool {
        if self.peilgebieden.is_empty() {
            return true;
        }

        // Start BFS vanaf het eerste peilgebied
        let start_id = self.peilgebieden.keys().next().unwrap();
        let mut bezocht = HashSet::new();
        let mut queue = vec![start_id.clone()];

        while let Some(current) = queue.pop() {
            if bezocht.contains(&current) {
                continue;
            }
            bezocht.insert(current.clone());

            // Voeg alle buren toe (ongeacht stroomrichting voor connectivity)
            for verbinding in self.verbindingen.values() {
                // Van current -> naar
                if verbinding.van_id == current && !bezocht.contains(&verbinding.naar_id) {
                    queue.push(verbinding.naar_id.clone());
                }
                // Van naar -> current (reverse traversal)
                if verbinding.naar_id == current && !bezocht.contains(&verbinding.van_id) {
                    queue.push(verbinding.van_id.clone());
                }
            }
        }

        bezocht.len() == self.peilgebieden.len()
    }

    /// Valideer de topologie.
    pub fn valideer(&self) -> Result<(), NetwerkFout> {
        if !self.is_verbonden() {
            return Err(NetwerkFout::NietVerbonden);
        }
        Ok(())
    }
}

impl Default for NetwerkTopologie {
    fn default() -> Self {
        Self::nieuw()
    }
}

/// Simulatiestatus voor multi-peilgebied netwerk.
#[derive(Debug, Clone)]
pub struct NetwerkSimulatie {
    /// Netwerktopologie
    pub topologie: NetwerkTopologie,
    /// Huidige waterstand per peilgebied
    pub waterstanden: HashMap<PeilgebiedId, f64>,
    /// Tijd in minuten
    pub tijd: f64,
}

impl NetwerkSimulatie {
    /// Maak een nieuwe simulatie vanuit topologie.
    pub fn nieuw(topologie: NetwerkTopologie) -> Result<Self, NetwerkFout> {
        topologie.valideer()?;

        // Initialiseer alle waterstanden op streefpeil
        let waterstanden = topologie
            .peilgebieden
            .iter()
            .map(|(id, config)| (id.clone(), config.streefpeil))
            .collect();

        Ok(Self {
            topologie,
            waterstanden,
            tijd: 0.0,
        })
    }

    /// Stel een specifieke startwaterstand in.
    pub fn met_start_waterstand(
        mut self,
        peilgebied_id: &str,
        waterstand: f64,
    ) -> Result<Self, NetwerkFout> {
        if !self.waterstanden.contains_key(peilgebied_id) {
            return Err(NetwerkFout::PeilgebiedNietGevonden {
                id: peilgebied_id.to_string(),
            });
        }

        self.waterstanden
            .insert(peilgebied_id.to_string(), waterstand);
        Ok(self)
    }

    /// Bereken waterstromen over alle verbindingen.
    pub fn bereken_stromen(
        &self,
        _regen_per_peilgebied: &HashMap<PeilgebiedId, f64>,
    ) -> Result<Vec<VerbindingStroom>, NetwerkFout> {
        let mut stromen = Vec::new();

        for verbinding in self.topologie.verbindingen.values() {
            let waterstand_van = *self
                .waterstanden
                .get(&verbinding.van_id)
                .ok_or_else(|| NetwerkFout::PeilgebiedNietGevonden {
                    id: verbinding.van_id.clone(),
                })?;
            let waterstand_naar = *self
                .waterstanden
                .get(&verbinding.naar_id)
                .ok_or_else(|| NetwerkFout::PeilgebiedNietGevonden {
                    id: verbinding.naar_id.clone(),
                })?;

            let stroom = match verbinding.verbinding_type {
                VerbindingType::Gemaal => {
                    // Actief transport: volledige capaciteit als richting Naar
                    let debiet = if verbinding.stroomrichting == Some(StroomRichting::Naar) {
                        verbinding.capaciteit
                    } else {
                        0.0
                    };
                    VerbindingStroom {
                        verbinding_id: verbinding.id.clone(),
                        debiet,
                        richting: StroomRichting::Naar,
                        benutting: debiet / verbinding.capaciteit,
                        actief: debiet > 0.0,
                    }
                }
                VerbindingType::Overstort => {
                    // Passieve stroming bij hoogwater boven drempel
                    let debiet = if waterstand_van > verbinding.overstort_drempel.unwrap() {
                        let niveauverschil = waterstand_van - waterstand_naar.max(verbinding.overstort_drempel.unwrap());
                        // Debiet schaalt met niveauverschil (weir flow vereenvoudigd)
                        (niveauverschil.sqrt() * verbinding.capaciteit).min(verbinding.capaciteit)
                    } else {
                        0.0
                    };
                    VerbindingStroom {
                        verbinding_id: verbinding.id.clone(),
                        debiet,
                        richting: StroomRichting::Naar,
                        benutting: debiet / verbinding.capaciteit,
                        actief: debiet > 0.0,
                    }
                }
                VerbindingType::Keerklep => {
                    // Eenrichting, maar alleen als van hoger is dan naar
                    let debiet = if waterstand_van > waterstand_naar {
                        let niveauverschil = waterstand_van - waterstand_naar;
                        (niveauverschil * verbinding.capaciteit).min(verbinding.capaciteit)
                    } else {
                        0.0
                    };
                    VerbindingStroom {
                        verbinding_id: verbinding.id.clone(),
                        debiet,
                        richting: StroomRichting::Naar,
                        benutting: debiet / verbinding.capaciteit,
                        actief: debiet > 0.0,
                    }
                }
                VerbindingType::OpenVerbinding => {
                    // Tweerichtingsstroming op basis van niveauverschil
                    let niveauverschil = waterstand_van - waterstand_naar;
                    let debiet = niveauverschil.abs() * verbinding.capaciteit;
                    let richting = if niveauverschil > 0.0 {
                        StroomRichting::Naar
                    } else {
                        StroomRichting::Terug
                    };
                    let debiet_beperkt = debiet.min(verbinding.capaciteit);
                    VerbindingStroom {
                        verbinding_id: verbinding.id.clone(),
                        debiet: debiet_beperkt,
                        richting,
                        benutting: debiet_beperkt / verbinding.capaciteit,
                        actief: debiet_beperkt > 0.0,
                    }
                }
            };

            stromen.push(stroom);
        }

        Ok(stromen)
    }

    /// Simuleer één tijdstap voor alle peilgebieden.
    pub fn simuleer_stap(
        &mut self,
        regen_per_peilgebied: &HashMap<PeilgebiedId, f64>,
        uitstroom_strategy: &dyn UitstroomStrategy,
    ) -> Result<Vec<PeilgebiedStatus>, NetwerkFout> {
        // Bereken verbindingstromen
        let stromen = self.bereken_stromen(regen_per_peilgebied)?;

        // Bereken inkomend/uitgaand debiet per peilgebied
        let mut inkomend_debiet: HashMap<PeilgebiedId, f64> = HashMap::new();
        let mut uitgaand_debiet: HashMap<PeilgebiedId, f64> = HashMap::new();

        for id in self.topologie.peilgebieden.keys() {
            inkomend_debiet.insert(id.clone(), 0.0);
            uitgaand_debiet.insert(id.clone(), 0.0);
        }

        for stroom in &stromen {
            let verbinding = self
                .topologie
                .verbindingen
                .get(&stroom.verbinding_id)
                .ok_or_else(|| NetwerkFout::VerbindingNietGevonden {
                    id: stroom.verbinding_id.clone(),
                })?;

            if stroom.actief {
                match stroom.richting {
                    StroomRichting::Naar => {
                        *uitgaand_debiet
                            .get_mut(&verbinding.van_id)
                            .unwrap() += stroom.debiet;
                        *inkomend_debiet
                            .get_mut(&verbinding.naar_id)
                            .unwrap() += stroom.debiet;
                    }
                    StroomRichting::Terug => {
                        *inkomend_debiet
                            .get_mut(&verbinding.van_id)
                            .unwrap() += stroom.debiet;
                        *uitgaand_debiet
                            .get_mut(&verbinding.naar_id)
                            .unwrap() += stroom.debiet;
                    }
                }
            }
        }

        // Update waterstanden per peilgebied
        let mut statuses = Vec::new();

        for (id, config) in &self.topologie.peilgebieden {
            let huidige_ws = *self.waterstanden.get(id).unwrap();
            let regen_intensiteit = regen_per_peilgebied.get(id).copied().unwrap_or(0.0);
            let inkomend = *inkomend_debiet.get(id).unwrap();
            let uitgaand = *uitgaand_debiet.get(id).unwrap();

            // Bepaal uitstroom debiet via strategy
            let uitstroom_debiet = uitstroom_strategy.bepaal_uitstroom(
                id,
                huidige_ws,
                config,
                regen_intensiteit,
                inkomend,
            );

            let pomp_actief = uitstroom_debiet > 0.001;

            // Bereken waterbalans
            let balans = calculate_water_balance(
                regen_intensiteit,
                config.oppervlakte,
                huidige_ws,
                uitgaand + uitstroom_debiet,
                config.verdamping,
                config.infiltratie,
            );

            // Update waterstand
            self.waterstanden
                .insert(id.clone(), balans.nieuwe_waterstand);

            statuses.push(PeilgebiedStatus {
                id: id.clone(),
                waterstand: huidige_ws,
                inkomend_debiet: inkomend,
                uitgaand_debiet: uitgaand,
                uitstroom_debiet,
                regen_intensiteit,
                pomp_actief,
            });
        }

        self.tijd += 1.0;
        Ok(statuses)
    }
}

/// Strategy voor bepalen van uitstroom naar boezem/extern.
pub trait UitstroomStrategy: Send + Sync {
    /// Bepaal uitstroom debiet (m³/s) voor peilgebied.
    fn bepaal_uitstroom(
        &self,
        peilgebied_id: &str,
        waterstand: f64,
        config: &PeilgebiedConfig,
        regen_intensiteit: f64,
        inkomend_debiet: f64,
    ) -> f64;
}

/// Simpele uitstroomstrategy: pomp als waterstand boven streefpeil.
#[derive(Debug, Clone, Copy)]
pub struct SimpeleUitstroomStrategy;

impl UitstroomStrategy for SimpeleUitstroomStrategy {
    fn bepaal_uitstroom(
        &self,
        _peilgebied_id: &str,
        waterstand: f64,
        config: &PeilgebiedConfig,
        _regen_intensiteit: f64,
        _inkomend_debiet: f64,
    ) -> f64 {
        if waterstand > config.streefpeil {
            config.max_uitstroom_debiet
        } else {
            0.0
        }
    }
}

/// Gebalanceerde uitstroomstrategy: verdeel waterlast over netwerk.
#[derive(Debug, Clone, Copy)]
pub struct GebalanceerdeUitstroomStrategy {
    /// Factor voor verdeling (0-1)
    pub balance_factor: f64,
}

impl Default for GebalanceerdeUitstroomStrategy {
    fn default() -> Self {
        Self { balance_factor: 0.5 }
    }
}

impl UitstroomStrategy for GebalanceerdeUitstroomStrategy {
    fn bepaal_uitstroom(
        &self,
        _peilgebied_id: &str,
        waterstand: f64,
        config: &PeilgebiedConfig,
        _regen_intensiteit: f64,
        inkomend_debiet: f64,
    ) -> f64 {
        let afwijking = waterstand - config.streefpeil;
        if afwijking <= 0.0 {
            return 0.0;
        }

        // Debiet schalen met afwijking en inkomende stroom
        let basis_debiet = config.max_uitstroom_debiet * (afwijking / config.marge).min(1.0);
        let gecorrigeerd = basis_debiet * self.balance_factor + inkomend_debiet * (1.0 - self.balance_factor);

        gecorrigeerd.min(config.max_uitstroom_debiet)
    }
}

/// Resultaat van netwerksimulatie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetwerkSimulatieResultaat {
    /// Tijdstappen
    pub tijdstappen: Vec<NetwerkTijdstap>,
    /// Totale kosten (ind van toepassing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totale_kosten: Option<f64>,
}

/// één tijdstap in netwerksimulatie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetwerkTijdstap {
    /// Tijd in minuten
    pub tijd: f64,
    /// Status per peilgebied
    pub statussen: HashMap<PeilgebiedId, PeilgebiedStatus>,
    /// Verbindingstromen
    pub stromen: Vec<VerbindingStroom>,
}

/// Run een netwerksimulatie voor gegeven regenscenario.
pub fn run_netwerksimulatie(
    topologie: &NetwerkTopologie,
    regen_scenario: &HashMap<PeilgebiedId, Vec<f64>>, // regen per uur per peilgebied
    duration_hours: usize,
    uitstroom_strategy: &dyn UitstroomStrategy,
) -> Result<NetwerkSimulatieResultaat, NetwerkFout> {
    let mut simulatie = NetwerkSimulatie::nieuw(topologie.clone())?;
    let mut tijdstappen = Vec::new();

    for uur in 0..duration_hours {
        for minuut in 0..60 {
            let _tijd_uur = uur + minuut / 60;
            let mut regen_per_peilgebied = HashMap::new();

            for (id, regen_per_uur) in regen_scenario {
                let regen = *regen_per_uur.get(uur).unwrap_or(&0.0);
                regen_per_peilgebied.insert(id.clone(), regen);
            }

            let statussen = simulatie.simuleer_stap(&regen_per_peilgebied, uitstroom_strategy)?;

            let stromen = simulatie.bereken_stromen(&regen_per_peilgebied)?;

            let status_map: HashMap<PeilgebiedId, PeilgebiedStatus> = statussen
                .into_iter()
                .map(|s| (s.id.clone(), s))
                .collect();

            tijdstappen.push(NetwerkTijdstap {
                tijd: simulatie.tijd,
                statussen: status_map,
                stromen,
            });
        }
    }

    Ok(NetwerkSimulatieResultaat {
        tijdstappen,
        totale_kosten: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maak_test_topologie() -> NetwerkTopologie {
        let mut topologie = NetwerkTopologie::nieuw();

        // Twee peilgebieden
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

        // Voeg verbinding toe om het netwerk verbonden te maken
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
    fn test_topologie_aanmaak() {
        let topologie = maak_test_topologie();
        assert_eq!(topologie.peilgebieden.len(), 2);
        // maak_test_topologie adds a connection, so it should be connected
        assert!(topologie.is_verbonden());
    }

    #[test]
    fn test_gemaal_verbinding() {
        let mut topologie = maak_test_topologie();

        let verbinding = Verbinding::nieuw_gemaal(
            "verbinding_ab".to_string(),
            "polder_a".to_string(),
            "polder_b".to_string(),
            0.3,
            2.0,
        )
        .unwrap();

        topologie.voeg_verbinding_toe(verbinding).unwrap();
        assert_eq!(topologie.verbindingen.len(), 1);
    }

    #[test]
    fn test_cyclische_verbinding_geweigerd() {
        // Maak een topologie zonder verbindingen
        let mut topologie = NetwerkTopologie::nieuw();

        topologie
            .voeg_peilgebied_toe(PeilgebiedConfig {
                id: "polder_a".to_string(),
                naam: None,
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
                naam: None,
                oppervlakte: 80_000.0,
                streefpeil: -0.60,
                marge: 0.20,
                maaiveld_niveau: 0.0,
                max_uitstroom_debiet: 0.4,
                verdamping: 0.0,
                infiltratie: 0.0,
            })
            .unwrap();

        topologie
            .voeg_verbinding_toe(Verbinding::nieuw_gemaal(
                "v1".to_string(),
                "polder_a".to_string(),
                "polder_b".to_string(),
                0.3,
                2.0,
            )
            .unwrap())
            .unwrap();

        let result = topologie.voeg_verbinding_toe(Verbinding::nieuw_gemaal(
            "v2".to_string(),
            "polder_b".to_string(),
            "polder_a".to_string(),
            0.3,
            2.0,
        ).unwrap());

        assert!(matches!(result, Err(NetwerkFout::CyclischeVerbinding { .. })));
    }

    #[test]
    fn test_ongeldige_verbinding() {
        let result = Verbinding::nieuw_gemaal(
            "v".to_string(),
            "polder_a".to_string(),
            "polder_a".to_string(),
            0.3,
            2.0,
        );

        assert!(matches!(result, Err(NetwerkFout::OngeldigeVerbinding { .. })));
    }

    #[test]
    fn test_simulatie_aanmaak() {
        let topologie = maak_test_topologie();
        let simulatie = NetwerkSimulatie::nieuw(topologie).unwrap();

        assert_eq!(simulatie.waterstanden.len(), 2);
        assert_eq!(simulatie.tijd, 0.0);
    }

    #[test]
    fn test_netwerk_is_verbonden() {
        // Maak een topologie zonder verbindingen
        let mut topologie = NetwerkTopologie::nieuw();

        topologie
            .voeg_peilgebied_toe(PeilgebiedConfig {
                id: "polder_a".to_string(),
                naam: None,
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
                naam: None,
                oppervlakte: 80_000.0,
                streefpeil: -0.60,
                marge: 0.20,
                maaiveld_niveau: 0.0,
                max_uitstroom_debiet: 0.4,
                verdamping: 0.0,
                infiltratie: 0.0,
            })
            .unwrap();

        assert!(!topologie.is_verbonden()); // Geen verbindingen

        topologie
            .voeg_verbinding_toe(
                Verbinding::nieuw_gemaal(
                    "v".to_string(),
                    "polder_a".to_string(),
                    "polder_b".to_string(),
                    0.3,
                    2.0,
                )
                .unwrap(),
            )
            .unwrap();

        assert!(topologie.is_verbonden());
    }

    #[test]
    fn test_peilgebied_config_grenzen() {
        let config = PeilgebiedConfig {
            id: "test".to_string(),
            naam: None,
            oppervlakte: 100_000.0,
            streefpeil: -0.60,
            marge: 0.20,
            maaiveld_niveau: 0.0,
            max_uitstroom_debiet: 0.5,
            verdamping: 0.0,
            infiltratie: 0.0,
        };

        assert_eq!(config.min_peil(), -0.80);
        // Use approximate comparison for floating point
        assert!((config.max_peil() - (-0.40)).abs() < 0.001);
        assert!(config.is_waterstand_geldig(-0.60));
        assert!(config.is_waterstand_geldig(-0.70));
        assert!(!config.is_waterstand_geldig(-0.30));
    }

    #[test]
    fn test_verbinding_stroomrichting() {
        assert!(VerbindingType::Gemaal.is_actief());
        assert!(!VerbindingType::Gemaal.is_passief());
        assert!(VerbindingType::Gemaal.is_eenrichting());

        assert!(!VerbindingType::Overstort.is_actief());
        assert!(VerbindingType::Overstort.is_passief());
        assert!(VerbindingType::Overstort.is_eenrichting());

        assert!(VerbindingType::OpenVerbinding.is_passief());
        assert!(!VerbindingType::OpenVerbinding.is_eenrichting());
    }

    #[test]
    fn test_simpele_uitstroom_strategy() {
        let strategy = SimpeleUitstroomStrategy;
        let config = PeilgebiedConfig {
            id: "test".to_string(),
            naam: None,
            oppervlakte: 100_000.0,
            streefpeil: -0.60,
            marge: 0.20,
            maaiveld_niveau: 0.0,
            max_uitstroom_debiet: 0.5,
            verdamping: 0.0,
            infiltratie: 0.0,
        };

        // Onder streefpeil: geen uitstroom
        let debiet = strategy.bepaal_uitstroom("test", -0.70, &config, 0.0, 0.0);
        assert_eq!(debiet, 0.0);

        // Boven streefpeil: vol debiet
        let debiet = strategy.bepaal_uitstroom("test", -0.50, &config, 0.0, 0.0);
        assert_eq!(debiet, 0.5);
    }

    #[test]
    fn test_netwerk_simulatie_twee_peilgebieden() {
        // Maak een netwerk van twee peilgebieden met verbinding
        let mut topologie = NetwerkTopologie::nieuw();

        topologie
            .voeg_peilgebied_toe(PeilgebiedConfig {
                id: "polder_noord".to_string(),
                naam: Some("Polder Noord".to_string()),
                oppervlakte: 150_000.0,
                streefpeil: -0.60,
                marge: 0.20,
                maaiveld_niveau: 0.0,
                max_uitstroom_debiet: 0.6,
                verdamping: 0.0,
                infiltratie: 0.0,
            })
            .unwrap();

        topologie
            .voeg_peilgebied_toe(PeilgebiedConfig {
                id: "polder_zuid".to_string(),
                naam: Some("Polder Zuid".to_string()),
                oppervlakte: 120_000.0,
                streefpeil: -0.60,
                marge: 0.20,
                maaiveld_niveau: 0.0,
                max_uitstroom_debiet: 0.5,
                verdamping: 0.0,
                infiltratie: 0.0,
            })
            .unwrap();

        // Gemaalverbinding van noord naar zuid
        topologie
            .voeg_verbinding_toe(
                Verbinding::nieuw_gemaal(
                    "verbinding_nz".to_string(),
                    "polder_noord".to_string(),
                    "polder_zuid".to_string(),
                    0.4,
                    1.5,
                )
                .unwrap(),
            )
            .unwrap();

        // Start simulatie
        let mut simulatie = NetwerkSimulatie::nieuw(topologie).unwrap();

        // Zet startwaterstand van polder_noord hoger
        simulatie = simulatie
            .met_start_waterstand("polder_noord", -0.40)
            .unwrap();

        // Simuleer één stap met regen
        let mut regen = HashMap::new();
        regen.insert("polder_noord".to_string(), 5.0); // 5 mm/uur
        regen.insert("polder_zuid".to_string(), 2.0); // 2 mm/uur

        let strategy = SimpeleUitstroomStrategy;
        let statussen = simulatie
            .simuleer_stap(&regen, &strategy)
            .unwrap();

        assert_eq!(statussen.len(), 2);
        assert!(simulatie.tijd > 0.0);

        // Check dat polder_noord water naar polder_zuid transporteert
        let stromen = simulatie.bereken_stromen(&regen).unwrap();
        let noord_naar_zuid: Vec<_> = stromen
            .iter()
            .filter(|s| s.verbinding_id == "verbinding_nz")
            .collect();

        assert_eq!(noord_naar_zuid.len(), 1);
        assert!(noord_naar_zuid[0].debiet > 0.0);
    }

    #[test]
    fn test_overstort_passieve_stroming() {
        // Test dat overstort alleen werkt bij hoogwater
        let mut topologie = NetwerkTopologie::nieuw();

        topologie
            .voeg_peilgebied_toe(PeilgebiedConfig {
                id: "hoog".to_string(),
                naam: None,
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
                id: "laag".to_string(),
                naam: None,
                oppervlakte: 80_000.0,
                streefpeil: -0.70,
                marge: 0.20,
                maaiveld_niveau: 0.0,
                max_uitstroom_debiet: 0.4,
                verdamping: 0.0,
                infiltratie: 0.0,
            })
            .unwrap();

        // Overstort met drempel op -0.50
        topologie
            .voeg_verbinding_toe(
                Verbinding::nieuw_overstort(
                    "overstort".to_string(),
                    "hoog".to_string(),
                    "laag".to_string(),
                    0.3,
                    -0.50,
                )
                .unwrap(),
            )
            .unwrap();

        let mut simulatie = NetwerkSimulatie::nieuw(topologie).unwrap();
        simulatie = simulatie
            .met_start_waterstand("hoog", -0.45)
            .unwrap();

        let regen: HashMap<String, f64> = HashMap::new();
        let stromen = simulatie.bereken_stromen(&regen).unwrap();

        // Moet een actieve overstroom hebben
        let overstort = &stromen[0];
        assert!(overstort.debiet > 0.0, "Overstort moet debiet hebben bij hoogwater");
        assert_eq!(overstort.richting, StroomRichting::Naar);
    }
}
