//! Bigify entry point: logging, Qt application, QML engine, and an orderly
//! shutdown of background services once the event loop ends.

mod bridge;
mod input;
mod logging;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

const MAIN_QML: &str = "qrc:/qt/qml/Bigify/qml/Main.qml";

fn main() {
    logging::init();
    log::info!("Bigify {} starting", env!("CARGO_PKG_VERSION"));

    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("Bigify"));
        app.as_mut().set_organization_name(&QString::from("Bigify"));
        app.as_mut()
            .set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    }

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(MAIN_QML));
    }

    let code = app.as_mut().map(|app| app.exec()).unwrap_or(1);

    // Stop the Spotify service (clean Spotify Connect disconnect) and the
    // controller thread while Qt objects still exist, then tear down QML.
    bridge::shutdown();
    drop(engine);
    drop(app);
    log::info!("Bigify exited with code {code}");
    std::process::exit(code);
}
