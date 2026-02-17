use opentelemetry::global;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

pub fn init_tracing(service_name: &'static str) {
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(service_name)
        .install_simple()
        .expect("Failed to install tracer");

    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();
}

pub fn init_metrics() {
    use prometheus::{Counter, Histogram, IntCounter, IntHistogram};
    use lazy_static::lazy_static;

    lazy_static! {
        static ref REQUEST_COUNT: IntCounter =
            IntCounter::new("http_requests_total", "Total HTTP requests").unwrap();
        static ref REQUEST_DURATION: Histogram =
            Histogram::with_histogram(Histogram::new("http_request_duration_seconds").unwrap());
    }
}
