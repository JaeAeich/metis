use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
pub use tracing;
pub use tracing_subscriber;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_tracing(service_name: &'static str) {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let use_json = std::env::var("LOG_FORMAT")
        .map(|v| v.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    let registry = tracing_subscriber::registry().with(filter);

    if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        match opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()
        {
            Ok(exporter) => {
                let resource =
                    opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                        "service.name",
                        service_name,
                    )]);
                let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
                    .with_resource(resource)
                    .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
                    .build();
                let tracer = tracer_provider.tracer(service_name);
                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
                if use_json {
                    registry.with(otel_layer).with(tracing_subscriber::fmt::layer().json()).init();
                } else {
                    registry.with(otel_layer).with(tracing_subscriber::fmt::layer()).init();
                }
                return;
            },
            Err(e) => {
                eprintln!(
                    "warn: failed to build OTEL exporter, falling back to local logging: {e}"
                );
            },
        }
    }
    if use_json {
        registry.with(tracing_subscriber::fmt::layer().json()).init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
}
