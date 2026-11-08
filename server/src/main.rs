use firstring::{calls::actor::App, config::Config, http::router};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "firstring=info".into()),
        )
        .init();
    let config = Config::from_env()?;
    let port = config.port;
    let mode = config.mode;
    let app = App::new(config).await?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port)).await?;
    tracing::info!(
        port,
        ?mode,
        "FirstRing listening; configured does not mean phone verified"
    );
    // Peer addresses let the operator console on this computer skip the token
    // (see `http::local_console`); tunnelled public requests still need it.
    axum::serve(
        listener,
        router(app).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
