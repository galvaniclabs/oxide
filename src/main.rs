//! Oxide IDE — a modern, terminal-native IDE built in Rust.

#[allow(clippy::unnecessary_wraps)] // main will propagate real errors once wired up
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Oxide IDE starting");

    Ok(())
}
