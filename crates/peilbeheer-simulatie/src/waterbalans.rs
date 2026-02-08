use peilbeheer_core::waterbalans::{SimulatieParams, SimulatieStap, WaterBalance};

use crate::pid::PidController;

/// Converteer mm/uur naar m³/s voor gegeven oppervlakte.
pub fn mm_per_uur_to_m3_per_sec(mm_per_uur: f64, oppervlakte_m2: f64) -> f64 {
    (mm_per_uur / 1000.0) * oppervlakte_m2 / 3600.0
}

/// Bereken waterbalans voor één tijdstap (1 minuut).
pub fn calculate_water_balance(
    regen_intensiteit: f64,
    oppervlakte: f64,
    huidige_waterstand: f64,
    gemaal_debiet: f64,
    verdamping: f64,
    infiltratie: f64,
) -> WaterBalance {
    let water_toevoer = mm_per_uur_to_m3_per_sec(regen_intensiteit, oppervlakte);
    let water_afvoer = gemaal_debiet;
    let water_verlies = mm_per_uur_to_m3_per_sec(verdamping + infiltratie, oppervlakte);
    let water_balans = water_toevoer - water_afvoer - water_verlies;
    let waterstand_verandering = (water_balans / oppervlakte) * 60.0; // m per minuut
    let nieuwe_waterstand = huidige_waterstand + waterstand_verandering;

    WaterBalance {
        water_toevoer,
        water_afvoer,
        water_verlies,
        water_balans,
        waterstand_verandering,
        nieuwe_waterstand,
    }
}

/// Bereken tijdreeks van waterstand over de simulatieperiode.
pub fn calculate_time_series(params: &SimulatieParams) -> Vec<SimulatieStap> {
    let mut tijdstappen = Vec::new();
    let mut huidige_waterstand = params.start_waterstand;
    let mut tijd = 0.0;
    let totale_duur = params.regen_duur + params.na_regen_duur;

    let mut pid = PidController::new(5.0, 0.05, 20.0);

    while tijd <= totale_duur {
        let is_regen = tijd <= params.regen_duur;
        let actuele_regen = if is_regen {
            params.regen_intensiteit
        } else {
            0.0
        };
        let mut actueel_debiet = params.gemaal_debiet;
        let mut pomp_aan = false;

        if params.smart_control && params.gemaal_debiet > 0.0 {
            let error = huidige_waterstand - params.streefpeil;
            let output = pid.update(error, params.tijd_stap);
            actueel_debiet = params.gemaal_debiet * output;
            pomp_aan = actueel_debiet > 0.001;
        }

        let balans = calculate_water_balance(
            actuele_regen,
            params.oppervlakte,
            huidige_waterstand,
            actueel_debiet,
            params.verdamping,
            params.infiltratie,
        );

        tijdstappen.push(SimulatieStap {
            tijd,
            waterstand: huidige_waterstand,
            water_toevoer: balans.water_toevoer,
            water_afvoer: balans.water_afvoer,
            water_verlies: balans.water_verlies,
            water_balans: balans.water_balans,
            is_regen,
            is_pomp_aan: pomp_aan,
        });

        huidige_waterstand = balans.nieuwe_waterstand;
        tijd += params.tijd_stap;
    }

    tijdstappen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mm_per_uur_to_m3_per_sec() {
        let result = mm_per_uur_to_m3_per_sec(10.0, 100_000.0);
        // 10 mm/uur = 0.01 m/uur * 100000 m² / 3600 s = 0.2778 m³/s
        assert!((result - 0.2778).abs() < 0.001);
    }

    #[test]
    fn test_water_balance_no_rain() {
        let result = calculate_water_balance(0.0, 100_000.0, -0.5, 0.0, 0.0, 0.0);
        assert!((result.water_toevoer).abs() < 0.0001);
        assert!((result.nieuwe_waterstand - (-0.5)).abs() < 0.0001);
    }

    #[test]
    fn test_water_balance_with_rain() {
        let result = calculate_water_balance(10.0, 100_000.0, -0.5, 0.0, 0.0, 0.0);
        assert!(result.water_toevoer > 0.0);
        assert!(result.nieuwe_waterstand > -0.5);
    }

    #[test]
    fn test_time_series_basic() {
        let params = SimulatieParams {
            start_waterstand: -0.5,
            regen_intensiteit: 10.0,
            regen_duur: 60.0,
            oppervlakte: 100_000.0,
            gemaal_debiet: 0.0,
            verdamping: 0.0,
            infiltratie: 0.0,
            na_regen_duur: 30.0,
            tijd_stap: 1.0,
            smart_control: false,
            streefpeil: 0.0,
            marge: 0.0,
            maaiveld_niveau: 0.0,
        };
        let result = calculate_time_series(&params);
        assert!(!result.is_empty());
        // Water level should rise during rain
        let last = result.last().unwrap();
        assert!(last.waterstand > params.start_waterstand);
    }

    #[test]
    fn test_time_series_with_pump() {
        let params = SimulatieParams {
            start_waterstand: -0.5,
            regen_intensiteit: 10.0,
            regen_duur: 60.0,
            oppervlakte: 100_000.0,
            gemaal_debiet: 0.5,
            verdamping: 0.0,
            infiltratie: 0.0,
            na_regen_duur: 30.0,
            tijd_stap: 1.0,
            smart_control: false,
            streefpeil: 0.0,
            marge: 0.0,
            maaiveld_niveau: 0.0,
        };
        let with_pump = calculate_time_series(&params);

        let params_no_pump = SimulatieParams {
            gemaal_debiet: 0.0,
            ..params
        };
        let without_pump = calculate_time_series(&params_no_pump);

        // Max water level should be lower with pump
        let max_with: f64 = with_pump
            .iter()
            .map(|s| s.waterstand)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_without: f64 = without_pump
            .iter()
            .map(|s| s.waterstand)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(max_with < max_without);
    }

    #[test]
    fn test_time_series_with_pid() {
        let params = SimulatieParams {
            start_waterstand: -0.5,
            regen_intensiteit: 10.0,
            regen_duur: 60.0,
            oppervlakte: 100_000.0,
            gemaal_debiet: 0.5,
            verdamping: 0.0,
            infiltratie: 0.0,
            na_regen_duur: 30.0,
            tijd_stap: 1.0,
            smart_control: true,
            streefpeil: -0.5,
            marge: 5.0,
            maaiveld_niveau: 0.0,
        };
        let result = calculate_time_series(&params);
        assert!(!result.is_empty());
    }
}
