use chrono::{NaiveDate, Utc};
use peilbeheer_core::energie::UurPrijs;
use serde::Deserialize;

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
pub async fn fetch_energieprijzen_vandaag() -> Result<Vec<UurPrijs>, String> {
    let today = Utc::now().date_naive();
    fetch_energieprijzen(today).await
}

/// Haal de EPEX-spotprijzen op voor een specifieke datum.
pub async fn fetch_energieprijzen(datum: NaiveDate) -> Result<Vec<UurPrijs>, String> {
    let from = format!("{}T00:00:00.000Z", datum);
    let till = format!(
        "{}T00:00:00.000Z",
        datum.succ_opt().ok_or("Ongeldige datum")?
    );

    let url = format!(
        "https://api.energyzero.nl/v1/energyprices?fromDate={}&tillDate={}&interval=4&usageType=1&inclBtw=true",
        from, till
    );

    tracing::debug!("EnergyZero request: {}", url);

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("EnergyZero request mislukt: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "EnergyZero API gaf status {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }

    let data: EnergyZeroResponse = response
        .json()
        .await
        .map_err(|e| format!("EnergyZero response parse mislukt: {e}"))?;

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
        return Err(format!(
            "EnergyZero retourneerde slechts {} uurprijzen (verwacht 24)",
            prijzen.len()
        ));
    }

    tracing::info!(
        "EnergyZero: {} uurprijzen opgehaald voor {}",
        prijzen.len(),
        datum
    );

    Ok(prijzen)
}
