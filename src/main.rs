use anyhow::Context;
use rimay_type::{application::Application, settings::get_configuration};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    let config = get_configuration().context("Failed to read configuration.")?;
    tracing::info!("{config:#?}");
    let app = Application::new(config)?;
    app.run().await
}

fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();
}
