use cosmogon_app::CosmogonApp;

fn main() {
    env_logger::init();
    log::info!("Starting Cosmogon v{}", env!("CARGO_PKG_VERSION"));
    log::info!("A real-time digital universe simulator");

    let app = CosmogonApp::new();
    app.run();
}
