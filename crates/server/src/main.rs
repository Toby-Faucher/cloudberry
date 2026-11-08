use server::{Config, app, station::now_ms};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();
    let cfg = Config::from_env();
    let (router, shared) = match app(cfg.clone()) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("startup failed: {e}");
            std::process::exit(1);
        }
    };
    let listener = match tokio::net::TcpListener::bind(&cfg.bind).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("cannot bind {}: {e}", cfg.bind);
            std::process::exit(1);
        }
    };
    tracing::info!("cloudberry listening on {}", cfg.bind);
    tokio::select! {
        r = axum::serve(listener, router) => if let Err(e) = r { tracing::error!("server: {e}") },
        _ = shutdown_signal() => tracing::info!("shutting down"),
    }
    // Save the live position so a restart resumes (paused) where we were.
    shared.persist(&shared.lock(), now_ms());
}

async fn shutdown_signal() {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}
