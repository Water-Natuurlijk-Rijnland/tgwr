use chrono::{NaiveDate, Utc};
use peilbeheer_core::energie::UurPrijs;
use serde::Deserialize;
use thiserror::Error;

/// Errors from EnergyZero API calls.
#[derive(Debug, Error)]
pub enum EnergyZeroError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("API returned error status {status}: {message}")]
    ApiError { status: reqwest::StatusCode, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Insufficient price data: expected 24 hours, got {0}")]
    InsufficientData(usize),

    #[error("Invalid date: {0}")]
    InvalidDate(String),
}

impl From<EnergyZeroError> for String {
    fn from(e: EnergyZeroError) -> Self {
        e.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct EnergyZeroResponse {
    #[serde(rename = "Prices")]
    prices: Vec<EnergyZeroPriceEntry>,
}

#[derive(Debug, Deserialize)]
struct EnergyZeroPriceEntry {
    #[serde(rename = "readingDate")]
    #[allow(dead_code)]
    reading_date: String,
    price: f64,
}

/// Haal de EPEX-spotprijzen op voor vandaag van de EnergyZero API.
/// Retourneert 24 UurPrijs entries (uur 0..23) met prijs incl. BTW in €/kWh.
#[allow(dead_code)]
pub async fn fetch_energieprijzen_vandaag() -> Result<Vec<UurPrijs>, EnergyZeroError> {
    let today = Utc::now().date_naive();
    fetch_energieprijzen(today).await
}

/// Haal de EPEX-spotprijzen op voor een specifieke datum.
pub async fn fetch_energieprijzen(datum: NaiveDate) -> Result<Vec<UurPrijs>, EnergyZeroError> {
    let from = format!("{}T00:00:00.000Z", datum);
    let till = format!(
        "{}T00:00:00.000Z",
        datum.succ_opt().ok_or_else(|| EnergyZeroError::InvalidDate("No next day".to_string()))?
    );

    let url = format!(
        "https://api.energyzero.nl/v1/energyprices?fromDate={}&tillDate={}&interval=4&usageType=1&inclBtw=true",
        from, till
    );

    tracing::debug!("EnergyZero request: {}", url);

    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let message = response.text().await.unwrap_or_default();
        return Err(EnergyZeroError::ApiError { status, message });
    }

    let data: EnergyZeroResponse = response.json().await?;

    // De API retourneert prijzen in €/kWh, gesorteerd op readingDate.
    // We nemen de eerste 24 entries en mappen ze naar uur 0..23.
    let prijzen: Vec<UurPrijs> = data
        .prices
        .into_iter()
        .take(24)
        .enumerate()
        .map(|(i, entry)| UurPrijs {
            uur: i as u8,
            prijs_eur_kwh: entry.price,
        })
        .collect();

    if prijzen.len() < 24 {
        return Err(EnergyZeroError::InsufficientData(prijzen.len()));
    }

    tracing::info!(
        "EnergyZero: {} uurprijzen opgehaald voor {}",
        prijzen.len(),
        datum
    );

    Ok(prijzen)
}

/// Fetch prices for multiple consecutive days.
pub async fn fetch_energieprijzen_multiple_days(start_date: NaiveDate, days: u8) -> Result<Vec<UurPrijs>, EnergyZeroError> {
    let mut all_prijzen = Vec::new();

    for day_offset in 0..days {
        let date = start_date
            .checked_add_days(chrono::Days::new(day_offset as u64))
            .ok_or_else(|| EnergyZeroError::InvalidDate("Invalid date offset".to_string()))?;

        match fetch_energieprijzen(date).await {
            Ok(mut prijzen) => {
                // Adjust hour numbers to be continuous across days
                for p in &mut prijzen {
                    p.uur += 24 * day_offset;
                }
                all_prijzen.extend(prijzen);
            }
            Err(EnergyZeroError::ApiError { .. }) => {
                // If we can't fetch a future day, that's okay - just return what we have
                tracing::warn!("Failed to fetch prices for day {}: API error, using {} hours total", day_offset, all_prijzen.len());
                break;
            }
            Err(e) => return Err(e),
        }
    }

    if all_prijzen.is_empty() {
        return Err(EnergyZeroError::InsufficientData(0));
    }

    Ok(all_prijzen)
}
