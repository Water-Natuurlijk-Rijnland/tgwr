pub mod drooglegging;
pub mod netwerk;
pub mod optimalisatie;
pub mod pid;
pub mod waterbalans;

pub use drooglegging::{calculate_drooglegging, find_minimum_debiet};
pub use netwerk::{
    GebalanceerdeUitstroomStrategy, NetwerkFout, NetwerkSimulatie, NetwerkSimulatieResultaat,
    NetwerkTijdstap, NetwerkTopologie, PeilgebiedConfig, PeilgebiedId, PeilgebiedStatus,
    SimpeleUitstroomStrategy, StroomRichting, UitstroomStrategy, Verbinding, VerbindingId,
    VerbindingStroom, VerbindingType,
};
pub use optimalisatie::optimize_pump_schedule;
pub use pid::PidController;
pub use waterbalans::{calculate_time_series, calculate_water_balance, mm_per_uur_to_m3_per_sec};
