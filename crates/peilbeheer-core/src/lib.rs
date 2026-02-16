pub mod asset;
pub mod dhydro;
pub mod energie;
pub mod gemaal;
pub mod hydronet;
pub mod peilgebied;
pub mod scenario;
pub mod sliding_window;
pub mod waterbalans;

pub use asset::AssetRegistratie;
pub use dhydro::{
    DhydroClient, DhydroConfig, DhydroError, DhydroModel, OAuthToken, Scenario,
    ScenarioParameters, ScenarioResult, ScenarioResults, ScenarioStatus,
    ScenarioSummary, TimeSeries, TimeSeriesAggregation, TimeSeriesPoint,
    TimeSeriesQuery,
};
pub use energie::{
    OptimalisatieParams, OptimalisatieResultaat, OptimalisatieUurResultaat,
    SimulatieStapUitgebreid, UurPrijs,
};
pub use gemaal::{Gemaal, GemaalSnapshot, GemaalStatus, GemaalTrends, StationSummary, TrendDirection, TrendInfo, TrendStrength};
pub use hydronet::{DataPoint, HydronetSeries};
pub use peilgebied::PeilgebiedInfo;
pub use scenario::{
    CloneScenarioRequest, CreateScenarioRequest, ExecutionStatus, ScenarioComparison,
    ScenarioComparisonItem, ScenarioComparisonStats, StoredScenario, StoredScenarioStatus,
    StoredScenarioResult, StoredTimeSeriesResult, UpdateScenarioRequest,
};
pub use sliding_window::{SlidingWindowProcessor, WindowStats};
pub use waterbalans::{SimulatieParams, SimulatieStap, WaterBalance};
