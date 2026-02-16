pub mod asset;
pub mod energie;
pub mod gemaal;
pub mod hydronet;
pub mod peilgebied;
pub mod sliding_window;
pub mod waterbalans;

pub use asset::AssetRegistratie;
pub use energie::{
    OptimalisatieParams, OptimalisatieResultaat, OptimalisatieUurResultaat,
    SimulatieStapUitgebreid, UurPrijs,
};
pub use gemaal::{Gemaal, GemaalSnapshot, GemaalStatus, GemaalTrends, StationSummary, TrendDirection, TrendInfo, TrendStrength};
pub use hydronet::{DataPoint, HydronetSeries};
pub use peilgebied::PeilgebiedInfo;
pub use sliding_window::{SlidingWindowProcessor, WindowStats};
pub use waterbalans::{SimulatieParams, SimulatieStap, WaterBalance};
