//! Optimization Runner API routes.
//!
//! Endpoints for pump scheduling optimization jobs and queue management.

use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

use peilbeheer_core::energie::*;

use crate::error::ApiError;
use crate::optimization_service::OptimizationService;

/// Request to create an optimization job.
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub name: String,
    pub peilgebied_id: String,
    pub params: OptimalisatieParams,
}

/// Response for job creation.
#[derive(Debug, Serialize)]
pub struct CreateJobResponse {
    pub job_id: String,
    pub status: String,
}

/// Get all jobs.
pub async fn list_jobs(
    Extension(service): Extension<Arc<OptimizationService>>,
) -> Result<Json<Vec<OptimizationJob>>, ApiError> {
    let jobs = service.list_jobs().await;
    Ok(Json(jobs))
}

/// Get a specific job.
pub async fn get_job(
    Extension(service): Extension<Arc<OptimizationService>>,
    Path(id): Path<String>,
) -> Result<Json<Option<OptimizationJob>>, ApiError> {
    Ok(Json(service.get_job(&id).await))
}

/// Create a new optimization job.
pub async fn create_job(
    Extension(service): Extension<Arc<OptimizationService>>,
    Json(req): Json<CreateJobRequest>,
) -> Result<Json<CreateJobResponse>, ApiError> {
    let job = OptimizationJob::new(req.name, req.peilgebied_id, req.params);

    match service.submit_job(job).await {
        Ok(job_id) => Ok(Json(CreateJobResponse {
            job_id,
            status: "queued".to_string(),
        })),
        Err(e) => {
            warn!("Failed to submit job: {}", e);
            Err(ApiError::Hydronet(format!("Failed to submit job: {}", e)))
        }
    }
}

/// Cancel a job.
pub async fn cancel_job(
    Extension(service): Extension<Arc<OptimizationService>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match service.cancel_job(&id).await {
        Ok(cancelled) => Ok(Json(serde_json::json!({
            "cancelled": cancelled,
            "job_id": id,
        }))),
        Err(e) => Err(ApiError::Hydronet(format!("Failed to cancel job: {}", e))),
    }
}

/// Get queue statistics.
pub async fn get_queue_stats(
    Extension(service): Extension<Arc<OptimizationService>>,
) -> Result<Json<QueueStats>, ApiError> {
    let stats = service.get_queue_stats().await;
    Ok(Json(stats))
}

/// Get price forecast.
pub async fn get_price_forecast(
    Extension(service): Extension<Arc<OptimizationService>>,
) -> Result<Json<PriceForecast>, ApiError> {
    match service.get_price_forecast(24).await {
        Ok(forecast) => Ok(Json(forecast)),
        Err(e) => Err(ApiError::Hydronet(format!("Failed to get forecast: {}", e))),
    }
}

/// Refresh price forecast (bypasses cache).
pub async fn refresh_price_forecast(
    Extension(service): Extension<Arc<OptimizationService>>,
) -> Result<Json<PriceForecast>, ApiError> {
    match service.refresh_price_forecast(48).await {
        Ok(forecast) => Ok(Json(forecast)),
        Err(e) => Err(ApiError::Hydronet(format!("Failed to refresh forecast: {}", e))),
    }
}

/// Run immediate optimization (synchronous).
pub async fn run_optimalisatie(
    Extension(service): Extension<Arc<OptimizationService>>,
    Json(mut params): Json<OptimalisatieParams>,
) -> Result<Json<OptimalisatieResultaat>, ApiError> {
    // Validate
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

    // If no prices provided, fetch from EnergyZero
    if params.prijzen.is_empty() {
        match service.get_price_forecast(24).await {
            Ok(forecast) => {
                params.prijzen = forecast.hourly_prices.iter()
                    .map(|hp| UurPrijs {
                        uur: (hp.hour_start.timestamp() / 3600) as u8,
                        prijs_eur_kwh: hp.price_eur_kwh,
                    })
                    .collect();
            }
            Err(e) => {
                return Err(ApiError::Hydronet(format!("EnergyZero: {}", e)));
            }
        }
    }

    // Run optimization
    let result = peilbeheer_simulatie::optimalisatie::optimize_pump_schedule(&params)
        .map_err(ApiError::Validation)?;

    Ok(Json(result))
}

/// Get energy prices (legacy endpoint - redirects to forecast).
pub async fn get_energieprijzen(
    Extension(service): Extension<Arc<OptimizationService>>,
) -> Result<Json<Vec<UurPrijs>>, ApiError> {
    match service.get_price_forecast(24).await {
        Ok(forecast) => {
            let prijzen: Vec<UurPrijs> = forecast.hourly_prices.iter()
                .map(|hp| UurPrijs {
                    uur: (hp.hour_start.timestamp() / 3600) as u8,
                    prijs_eur_kwh: hp.price_eur_kwh,
                })
                .collect();
            Ok(Json(prijzen))
        }
        Err(e) => Err(ApiError::Hydronet(format!("Failed to get prices: {}", e))),
    }
}
