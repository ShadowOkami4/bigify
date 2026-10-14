//! Bigify's Spotify service.
//!
//! The service runs on its own thread with a small Tokio runtime and talks to
//! the UI only through [`bigify_core::Command`]s (in) and
//! [`bigify_core::Event`]s (out), so the UI thread never blocks on network
//! or audio work.

mod artwork;
mod engine;
mod errors;
mod metadata;
mod player_events;
pub mod userauth;
pub mod webapi;

use std::thread::JoinHandle;

use bigify_core::{AuthState, Command, Event};
use tokio::sync::mpsc;

pub use engine::{EngineConfig, EventSink};

pub struct ServiceHandle {
    tx: mpsc::UnboundedSender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl ServiceHandle {
    /// Queue a command. Returns false if the service has stopped.
    pub fn send(&self, cmd: Command) -> bool {
        self.tx.send(cmd).is_ok()
    }

    /// Stop the service and wait for a clean librespot shutdown.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                log::error!("Spotify service thread panicked during shutdown");
            }
        }
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Start the service thread.
pub fn spawn(config: EngineConfig, sink: EventSink) -> ServiceHandle {
    let (tx, rx) = mpsc::unbounded_channel();
    let thread = std::thread::Builder::new()
        .name("bigify-spotify".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("bigify-net")
                .enable_all()
                .build();
            let runtime = match runtime {
                Ok(rt) => rt,
                Err(e) => {
                    sink(Event::Auth(AuthState::Failed {
                        message: format!("Could not start network runtime: {e}"),
                    }));
                    return;
                }
            };
            runtime.block_on(async move {
                match engine::Engine::new(config, sink.clone()) {
                    Ok(engine) => engine.run(rx).await,
                    Err(e) => sink(Event::Auth(AuthState::Failed { message: e.message })),
                }
            });
            // Give background tasks (player event forwarding, requests) a
            // bounded time to finish instead of hanging on exit.
            runtime.shutdown_timeout(std::time::Duration::from_secs(2));
        })
        .expect("failed to spawn Spotify service thread");
    ServiceHandle {
        tx,
        thread: Some(thread),
    }
}
