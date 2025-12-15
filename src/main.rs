use anyhow::Context;
use rimay_type::{application::Application, settings::get_configuration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = get_configuration().context("Failed to read configuration.")?;
    let app = Application::new(config)?;
    app.run().await
}
