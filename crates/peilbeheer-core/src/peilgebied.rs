use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeilgebiedInfo {
    pub code: String,
    pub naam: Option<String>,
    pub zomerpeil: Option<f64>,
    pub winterpeil: Option<f64>,
    pub vastpeil: Option<f64>,
    pub oppervlakte: Option<f64>,
    pub soortafwatering: Option<String>,
}
