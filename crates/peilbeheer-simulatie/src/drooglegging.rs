use peilbeheer_core::waterbalans::{DroogleggingResult, MinimumDebietResult, SimulatieParams, SimulatieStap};

use crate::waterbalans::calculate_time_series;

/// Bereken drooglegging en overschrijding. Marge in cm.
pub fn calculate_drooglegging(
    maaiveld_niveau: f64,
    waterstand: f64,
    streefpeil: f64,
    marge: f64,
) -> DroogleggingResult {
    let drooglegging = maaiveld_niveau - waterstand;
    let streef_drooglegging = maaiveld_niveau - streefpeil;
    let minimale_drooglegging = streef_drooglegging - (marge / 100.0);
    let overschrijding = minimale_drooglegging - drooglegging;

    DroogleggingResult {
        drooglegging,
        streef_drooglegging,
        minimale_drooglegging,
        overschrijding,
        overschrijding_cm: overschrijding * 100.0,
    }
}

/// Vind maximale waterstand in een tijdreeks.
pub fn find_max_waterstand(tijdstappen: &[SimulatieStap]) -> Option<(f64, f64)> {
    tijdstappen
        .iter()
        .max_by(|a, b| a.waterstand.partial_cmp(&b.waterstand).unwrap())
        .map(|s| (s.waterstand, s.tijd))
}

/// Vind minimaal benodigd debiet zodat drooglegging binnen marge blijft.
pub fn find_minimum_debiet(params: &SimulatieParams) -> MinimumDebietResult {
    let stap_grootte = 0.1;
    let max_debiet = 100.0;

    // Startschatting
    let geschat = mm_per_uur_to_m3_per_sec_inline(params.regen_intensiteit, params.oppervlakte);
    let mut debiet = (geschat * 0.8).max(0.0);
    let mut last_max_overschrijding = 0.0;

    while debiet <= max_debiet {
        let test_params = SimulatieParams {
            gemaal_debiet: debiet,
            smart_control: false,
            ..params.clone()
        };

        let tijdstappen = calculate_time_series(&test_params);
        let mut max_ov: f64 = 0.0;

        for stap in &tijdstappen {
            let d = calculate_drooglegging(
                params.maaiveld_niveau,
                stap.waterstand,
                params.streefpeil,
                params.marge,
            );
            if d.overschrijding > 0.0 {
                max_ov = max_ov.max(d.overschrijding);
            }
        }

        if max_ov == 0.0 {
            return MinimumDebietResult {
                minimaal_debiet: Some(debiet),
                max_overschrijding: 0.0,
                success: true,
                message: None,
                tijdstappen,
            };
        }

        last_max_overschrijding = max_ov;
        debiet += stap_grootte;
    }

    // Geen oplossing gevonden
    let final_params = SimulatieParams {
        gemaal_debiet: max_debiet,
        smart_control: false,
        ..params.clone()
    };
    let tijdstappen = calculate_time_series(&final_params);

    MinimumDebietResult {
        minimaal_debiet: None,
        max_overschrijding: last_max_overschrijding * 100.0,
        success: false,
        message: Some(format!(
            "Geen debiet gevonden binnen {} m³/s",
            max_debiet
        )),
        tijdstappen,
    }
}

fn mm_per_uur_to_m3_per_sec_inline(mm_per_uur: f64, oppervlakte_m2: f64) -> f64 {
    (mm_per_uur / 1000.0) * oppervlakte_m2 / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drooglegging_basic() {
        let result = calculate_drooglegging(0.0, -0.5, -0.6, 5.0);
        // drooglegging = 0.0 - (-0.5) = 0.5 m
        assert!((result.drooglegging - 0.5).abs() < 0.001);
        // streef_drooglegging = 0.0 - (-0.6) = 0.6 m
        assert!((result.streef_drooglegging - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_drooglegging_geen_overschrijding() {
        // Waterstand ver onder maaiveld
        let result = calculate_drooglegging(0.0, -1.0, -0.6, 5.0);
        assert!(result.overschrijding < 0.0); // Negatief = geen overschrijding
    }

    #[test]
    fn test_find_max_waterstand() {
        let stappen = vec![
            SimulatieStap {
                tijd: 0.0,
                waterstand: -0.5,
                water_toevoer: 0.0,
                water_afvoer: 0.0,
                water_verlies: 0.0,
                water_balans: 0.0,
                is_regen: true,
                is_pomp_aan: false,
            },
            SimulatieStap {
                tijd: 1.0,
                waterstand: -0.3,
                water_toevoer: 0.0,
                water_afvoer: 0.0,
                water_verlies: 0.0,
                water_balans: 0.0,
                is_regen: true,
                is_pomp_aan: false,
            },
            SimulatieStap {
                tijd: 2.0,
                waterstand: -0.4,
                water_toevoer: 0.0,
                water_afvoer: 0.0,
                water_verlies: 0.0,
                water_balans: 0.0,
                is_regen: false,
                is_pomp_aan: false,
            },
        ];
        let (max_ws, tijd) = find_max_waterstand(&stappen).unwrap();
        assert!((max_ws - (-0.3)).abs() < 0.001);
        assert!((tijd - 1.0).abs() < 0.001);
    }
}
