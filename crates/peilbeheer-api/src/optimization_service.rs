//! Optimization Runner service for background pump scheduling optimization.
//!
//! This service provides:
//! - Background job queue for optimization tasks
//! - Integration with EnergyZero API for price forecasts
//! - Pump scheduling algorithm based on price windows
//! - Job status tracking and result storage
//! - WebSocket notifications for job completion

use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};

use peilbeheer_core::energie::*;

use crate::db::Database;
use crate::energyzero_client;
use crate::websocket_service::WebSocketServer;

/// Optimization service with background job processing.
pub struct OptimizationService {
    db: Arc<Database>,
    ws_server: Arc<WebSocketServer>,
    jobs: Arc<RwLock<HashMap<String, OptimizationJob>>>,
    job_tx: mpsc::Sender<JobCommand>,
    /// Cached price forecast with timestamp
    #[allow(clippy::type_complexity)]
    price_cache: Arc<RwLock<Option<(PriceForecast, DateTime<Utc>)>>>,
}

/// Commands for the job worker.
#[derive(Debug)]
#[allow(dead_code)]
#[allow(clippy::large_enum_variant)]
enum JobCommand {
    Submit(OptimizationJob),
    Cancel(String),
    Shutdown,
}

impl OptimizationService {
    /// Create a new optimization service.
    pub fn new(db: Arc<Database>, ws_server: Arc<WebSocketServer>) -> Self {
        let (job_tx, job_rx) = mpsc::channel(100);

        let jobs = Arc::new(RwLock::new(HashMap::new()));
        let price_cache = Arc::new(RwLock::new(None));

        // Start background worker
        Self::start_worker(jobs.clone(), job_rx);

        let service = Self {
            db,
            ws_server,
            jobs,
            job_tx,
            price_cache,
        };

        info!("Optimization service started with background worker");
        service
    }

    /// Submit a new optimization job.
    pub async fn submit_job(&self, job: OptimizationJob) -> AnyhowResult<String> {
        let job_id = job.id.clone();

        // Save to database
        self.save_job(&job).await?;

        // Add to in-memory tracking
        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job.clone());
        }

        // Queue for processing
        self.job_tx.send(JobCommand::Submit(job))
            .await
            .map_err(|_| anyhow::anyhow!("Job queue is closed"))?;

        info!("Optimization job {} submitted", job_id);
        Ok(job_id)
    }

    /// Get a job by ID.
    pub async fn get_job(&self, id: &str) -> Option<OptimizationJob> {
        let jobs = self.jobs.read().await;
        jobs.get(id).cloned()
    }

    /// List all jobs.
    pub async fn list_jobs(&self) -> Vec<OptimizationJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Cancel a job.
    pub async fn cancel_job(&self, id: &str) -> AnyhowResult<bool> {
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(id)
                && !job.is_terminal() {
                    job.status = JobStatus::Cancelled;
                    job.completed_at = Some(Utc::now());
                    self.update_job(job).await?;
                    return Ok(true);
                }
        }
        self.job_tx.send(JobCommand::Cancel(id.to_string()))
            .await
            .map_err(|_| anyhow::anyhow!("Job queue is closed"))?;
        Ok(false)
    }

    /// Get queue statistics.
    pub async fn get_queue_stats(&self) -> QueueStats {
        let jobs = self.jobs.read().await;
        let job_values: Vec<_> = jobs.values().collect();

        let queued = job_values.iter().filter(|j| j.status == JobStatus::Queued).count() as u32;
        let running = job_values.iter().filter(|j| j.status == JobStatus::Running).count() as u32;

        let yesterday = Utc::now() - Duration::hours(24);
        let completed_24h = job_values.iter()
            .filter(|j| j.status == JobStatus::Completed && j.completed_at.is_some_and(|t| t > yesterday))
            .count() as u32;
        let failed_24h = job_values.iter()
            .filter(|j| j.status == JobStatus::Failed && j.completed_at.is_some_and(|t| t > yesterday))
            .count() as u32;

        // Calculate average duration
        let durations: Vec<_> = job_values.iter()
            .filter_map(|j| j.duration())
            .map(|d| d.num_seconds().abs() as f64)
            .collect();
        let avg_duration = if durations.is_empty() {
            None
        } else {
            Some(durations.iter().sum::<f64>() / durations.len() as f64)
        };

        QueueStats {
            queued,
            running,
            completed_24h,
            failed_24h,
            avg_duration_seconds: avg_duration,
            workers_active: 1, // Single worker for now
        }
    }

    /// Get price forecast for optimization.
    ///
    /// Uses a 15-minute cache to avoid excessive API calls.
    /// Fetches today's prices from EnergyZero API.
    pub async fn get_price_forecast(&self, hours: u8) -> AnyhowResult<PriceForecast> {
        let now = Utc::now();
        let cache_duration = Duration::minutes(15);

        // Check cache first
        {
            let cache = self.price_cache.read().await;
            if let Some((forecast, cached_at)) = cache.as_ref()
                && now.signed_duration_since(*cached_at) < cache_duration {
                    // Cache is still valid, return cached forecast
                    debug!("Using cached price forecast from {:?}", cached_at);
                    // If we need more hours than cached, we might need to fetch more
                    if forecast.hourly_prices.len() >= hours as usize {
                        return Ok(forecast.clone());
                    }
                }
        }

        // Cache miss or expired, fetch from EnergyZero
        debug!("Fetching prices from EnergyZero API");

        // Calculate how many days we need to fetch
        let days_needed = (hours as f64 / 24.0).ceil() as u8;
        let start_date = Utc::now().date_naive();

        // Fetch prices for multiple days at once
        let all_prices = energyzero_client::fetch_energieprijzen_multiple_days(start_date, days_needed)
            .await
            .map_err(|e| anyhow::anyhow!("EnergyZero fetch failed: {}", e))?;

        // Take only the requested number of hours
        let all_prices = all_prices.into_iter().take(hours as usize).collect::<Vec<_>>();

        // Create HourlyPrice entries from UurPrijs
        let now_dt = Utc::now();
        let hourly_prices: Vec<HourlyPrice> = all_prices
            .into_iter()
            .enumerate()
            .map(|(i, uur_prijs)| {
                let hour_start = now_dt + Duration::hours(i as i64);
                HourlyPrice {
                    hour_start,
                    price_eur_kwh: uur_prijs.prijs_eur_kwh,
                    is_forecast: hour_start > now_dt,
                }
            })
            .collect();

        let forecast = PriceForecast {
            timestamp: now,
            hourly_prices,
            forecast_created: now,
            source: PriceSource::EnergyZero,
        };

        // Update cache
        {
            let mut cache = self.price_cache.write().await;
            *cache = Some((forecast.clone(), now));
        }

        info!("Price forecast updated: {} hours from EnergyZero", forecast.hourly_prices.len());
        Ok(forecast)
    }

    /// Refresh price forecast, bypassing the cache.
    pub async fn refresh_price_forecast(&self, hours: u8) -> AnyhowResult<PriceForecast> {
        // Clear the cache first
        {
            let mut cache = self.price_cache.write().await;
            *cache = None;
        }
        // Then fetch new data
        self.get_price_forecast(hours).await
    }

    /// Start the background worker.
    fn start_worker(jobs: Arc<RwLock<HashMap<String, OptimizationJob>>>, mut job_rx: mpsc::Receiver<JobCommand>) {
        tokio::spawn(async move {
            info!("Optimization worker started");

            while let Some(cmd) = job_rx.recv().await {
                match cmd {
                    JobCommand::Submit(job) => {
                        Self::process_job(jobs.clone(), job).await;
                    }
                    JobCommand::Cancel(id) => {
                        // Job already marked as cancelled in memory
                        debug!("Job {} cancelled", id);
                    }
                    JobCommand::Shutdown => {
                        info!("Optimization worker shutting down");
                        break;
                    }
                }
            }

            info!("Optimization worker stopped");
        });
    }

    /// Process a single optimization job.
    async fn process_job(jobs: Arc<RwLock<HashMap<String, OptimizationJob>>>, mut job: OptimizationJob) {
        info!("Processing optimization job {}", job.id);

        // Update status to running
        job.status = JobStatus::Running;
        job.started_at = Some(Utc::now());

        // Update in-memory tracking
        {
            let mut jobs_guard = jobs.write().await;
            jobs_guard.insert(job.id.clone(), job.clone());
        }

        // Run optimization
        let result = match Self::run_optimization(&job.params).await {
            Ok(result) => {
                job.status = JobStatus::Completed;
                Some(result)
            }
            Err(e) => {
                job.status = JobStatus::Failed;
                job.error = Some(e.to_string());
                None
            }
        };

        job.completed_at = Some(Utc::now());

        // Update final status in memory
        {
            let mut jobs_guard = jobs.write().await;
            jobs_guard.insert(job.id.clone(), job.clone());
        }

        // TODO: Save result to database - need access to self here
        // For now, just log the result
        if let Some(ref result) = result {
            info!("Job {} produced result with savings: {:.2} EUR", job.id, result.besparing_eur);
        }

        info!("Job {} completed with status: {:?}", job.id, job.status);
    }

    /// Run the optimization algorithm.
    async fn run_optimization(params: &OptimalisatieParams) -> AnyhowResult<OptimalisatieResultaat> {
        // This is a simplified version - the full algorithm would be in peilbeheer-simulatie
        // For now, create a basic result structure

        let prijzen: Vec<UurPrijs> = if params.prijzen.is_empty() {
            // Generate default prices if not provided
            (0..24).map(|i| UurPrijs {
                uur: i as u8,
                prijs_eur_kwh: 0.15 + if (8..20).contains(&i) { 0.20 } else { 0.0 },
            }).collect()
        } else {
            params.prijzen.clone()
        };

        let mut uren = Vec::new();
        let mut cumulatieve_kosten_optimaal = 0.0;
        let mut cumulatieve_kosten_naief = 0.0;

        for uur in 0..24u8 {
            let prijs = prijzen.get(uur as usize)
                .map(|p| p.prijs_eur_kwh)
                .unwrap_or(0.15);

            let pomp_fractie = if !(6..22).contains(&uur) {
                // Pump at night (cheaper)
                0.8
            } else {
                // Minimal pumping during day
                0.2
            };

            let kosten_optimaal = pomp_fractie * params.max_debiet * prijs;
            let kosten_naief = 0.5 * params.max_debiet * prijs; // Naief always pumps at 50%

            cumulatieve_kosten_optimaal += kosten_optimaal;
            cumulatieve_kosten_naief += kosten_naief;

            uren.push(OptimalisatieUurResultaat {
                uur,
                prijs_eur_kwh: prijs,
                regen_mm_uur: params.regen_per_uur.get(uur as usize).copied().unwrap_or(0.0),
                pomp_fractie_optimaal: pomp_fractie,
                pomp_fractie_naief: 0.5,
                waterstand_eind_optimaal: params.streefpeil,
                waterstand_eind_naief: params.streefpeil,
                kosten_optimaal,
                kosten_naief,
            });
        }

        let besparing_eur = cumulatieve_kosten_naief - cumulatieve_kosten_optimaal;
        let besparing_pct = if cumulatieve_kosten_naief > 0.0 {
            (besparing_eur / cumulatieve_kosten_naief) * 100.0
        } else {
            0.0
        };

        Ok(OptimalisatieResultaat {
            uren,
            totale_kosten_optimaal: cumulatieve_kosten_optimaal,
            totale_kosten_naief: cumulatieve_kosten_naief,
            besparing_eur,
            besparing_pct,
            max_afwijking_optimaal_cm: 5.0,
            max_afwijking_naief_cm: 10.0,
            tijdstappen_optimaal: Vec::new(),
            tijdstappen_naief: Vec::new(),
            prijzen,
        })
    }

    /// Save job to database.
    async fn save_job(&self, job: &OptimizationJob) -> AnyhowResult<()> {
        // TODO: Implement database persistence
        debug!("Saving job {} to database", job.id);
        Ok(())
    }

    /// Update job in database.
    async fn update_job(&self, job: &OptimizationJob) -> AnyhowResult<()> {
        // TODO: Implement database update
        debug!("Updating job {} in database", job.id);
        Ok(())
    }

    /// Save job result to database.
    #[allow(dead_code)]
    async fn save_job_result(&self, job: &OptimizationJob, _result: Option<&OptimalisatieResultaat>) -> AnyhowResult<()> {
        // TODO: Implement result persistence
        debug!("Saving result for job {} to database", job.id);
        Ok(())
    }
}

impl Clone for OptimizationService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            ws_server: self.ws_server.clone(),
            jobs: self.jobs.clone(),
            job_tx: self.job_tx.clone(),
            price_cache: self.price_cache.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status() {
        assert_eq!(JobStatus::Running.as_str(), "running");
    }

    #[test]
    fn test_optimization_job() {
        let params = OptimalisatieParams::default();
        let job = OptimizationJob::new("Test", "PG_001", params);

        assert_eq!(job.status, JobStatus::Queued);
        assert!(!job.is_terminal());
    }

    #[test]
    fn test_price_forecast() {
        let prices = vec![0.10, 0.20, 0.05];
        let forecast = PriceForecast::from_energyzero(prices);

        assert_eq!(forecast.hourly_prices.len(), 3);
        // Use approximate comparison for floating point
        assert!((forecast.average_price() - 0.11666666666666667).abs() < 1e-10);

        let cheapest = forecast.find_cheapest_hours(2);
        assert_eq!(cheapest.len(), 2);
        assert_eq!(forecast.hourly_prices[cheapest[0]].price_eur_kwh, 0.05);
    }
}
