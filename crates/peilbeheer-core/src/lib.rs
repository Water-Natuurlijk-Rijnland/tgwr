pub mod alert;
pub mod asset;
pub mod auth;
pub mod dhydro;
pub mod energie;
pub mod fews;
pub mod gemaal;
pub mod hydronet;
pub mod peilgebied;
pub mod scenario;
pub mod sliding_window;
pub mod timeseries;
pub mod waterbalans;
pub mod websocket;

pub use asset::AssetRegistratie;
pub use auth::{
    ChangePasswordRequest, Claims, CreateUserRequest, LoginRequest, LoginResponse,
    Permission, Role, UpdateUserRequest, User, UserInfo,
};
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
pub use websocket::{
    AlertSeverity as WsAlertSeverity, SubscribeRequest, TimeSeriesPoint as WsTimeSeriesPoint,
    UnsubscribeRequest, WsMessage,
};
pub use alert::{
    AcknowledgeAlertRequest, Alert, AlertCategory, AlertCondition, AlertQuery, AlertRule,
    AlertSeverity, AlertStats, AlertStatus, AlertValue,
    AggregationFunction as AlertAggregationFunction, ComparisonOperator,
    ConditionLogic, ConditionResult, CreateAlertRuleRequest, EvaluationContext,
    NotificationChannel, RuleEvaluationResult, RuleId as AlertRuleId, RuleTriggerCount,
    TimeSeriesValue, UpdateAlertRuleRequest,
};
pub use fews::{
    FewsConfig, FewsLocation, FewsModuleInstance, FewsParameter,
    FewsSyncConfig, FewsSyncRequest, FewsSyncResult, FewsTimeSeries, FewsTimeSeriesHeader,
    FewsTimeSeriesId, FewsTimeSeriesPoint, FewsTimeSeriesQuery, FewsTimeSeriesResponse,
    FewsTimeStep, FewsValueType,
};
pub use waterbalans::{SimulatieParams, SimulatieStap, WaterBalance};
pub use timeseries::{
    AggregatedSeries, AggregationFunction as TsAggregationFunction, AggregationLevel,
    AggregationMetadata, DownsampleConfig, FillMethod, GapAnalysisResult, QualityFlag,
    TimeSeriesCatalogEntry, TimeSeriesDataPoint, TimeSeriesId, TimeSeriesMetadata,
    TimeSeriesQuery as TsQuery, TimeSeriesSourceType, TimeSeriesWriteBatch, TimeSeriesWriteResult,
};
