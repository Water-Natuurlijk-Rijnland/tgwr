use axum::Json;

use crate::error::ApiError;

use peilbeheer_core::waterbalans::SimulatieParams;
use peilbeheer_simulatie::waterbalans::calculate_time_series;

/// POST /api/simulatie - Voer een waterbalans simulatie uit.
///
/// Verwacht een JSON body met SimulatieParams.
/// Retourneert een tijdreeks van waterstandberekeningen.
pub async fn run_simulatie(
    Json(params): Json<SimulatieParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validatie
    if params.oppervlakte <= 0.0 {
        return Err(ApiError::Validation(
            "oppervlakte moet groter zijn dan 0".to_string(),
        ));
    }
    if params.regen_intensiteit < 0.0 {
        return Err(ApiError::Validation(
            "regen_intensiteit mag niet negatief zijn".to_string(),
        ));
    }
    if params.regen_duur <= 0.0 {
        return Err(ApiError::Validation(
            "regen_duur moet groter zijn dan 0".to_string(),
        ));
    }

    let tijdstappen = calculate_time_series(&params);

    // Bereken samenvatting
    let max_waterstand = tijdstappen
        .iter()
        .map(|s| s.waterstand)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_waterstand = tijdstappen
        .iter()
        .map(|s| s.waterstand)
        .fold(f64::INFINITY, f64::min);

    // Optioneel: drooglegging berekenen als maaiveld is opgegeven
    let drooglegging = if params.maaiveld_niveau != 0.0 {
        Some(peilbeheer_simulatie::drooglegging::calculate_drooglegging(
            params.maaiveld_niveau,
            max_waterstand,
            params.streefpeil,
            params.marge,
        ))
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "params": params,
        "tijdstappen": tijdstappen,
        "samenvatting": {
            "max_waterstand": max_waterstand,
            "min_waterstand": min_waterstand,
            "aantal_stappen": tijdstappen.len(),
        },
        "drooglegging": drooglegging,
    })))
}
