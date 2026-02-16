use axum::Json;

use crate::energyzero_client;
use crate::error::ApiError;

use peilbeheer_core::energie::{OptimalisatieParams, OptimalisatieResultaat, UurPrijs};
use peilbeheer_simulatie::optimalisatie::optimize_pump_schedule;

/// GET /api/energieprijzen — haal vandaag's EPEX-spotprijzen op.
pub async fn get_energieprijzen() -> Result<Json<Vec<UurPrijs>>, ApiError> {
    let prijzen = energyzero_client::fetch_energieprijzen_vandaag()
        .await
        .map_err(|e| ApiError::Hydronet(format!("EnergyZero: {e}")))?;

    Ok(Json(prijzen))
}

/// POST /api/optimalisatie — voer DP-optimalisatie uit.
///
/// Als `prijzen` leeg zijn in de request, worden ze opgehaald via de EnergyZero API.
pub async fn run_optimalisatie(
    Json(mut params): Json<OptimalisatieParams>,
) -> Result<Json<OptimalisatieResultaat>, ApiError> {
    // Validatie
    if params.oppervlakte <= 0.0 {
        return Err(ApiError::Validation(
            "oppervlakte moet groter zijn dan 0".into(),
        ));
    }
    if params.max_debiet <= 0.0 {
        return Err(ApiError::Validation(
            "max_debiet moet groter zijn dan 0".into(),
        ));
    }
    if params.regen_per_uur.len() != 24 {
        return Err(ApiError::Validation(format!(
            "regen_per_uur moet 24 waarden bevatten, maar bevat {}",
            params.regen_per_uur.len()
        )));
    }

    // Als er geen prijzen meegestuurd zijn, haal ze op
    if params.prijzen.is_empty() {
        params.prijzen = energyzero_client::fetch_energieprijzen_vandaag()
            .await
            .map_err(|e| ApiError::Hydronet(format!("EnergyZero: {e}")))?;
    }

    let resultaat = optimize_pump_schedule(&params)
        .map_err(|e| ApiError::Validation(e))?;

    Ok(Json(resultaat))
}
