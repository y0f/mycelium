use std::path::PathBuf;
use std::sync::Arc;

use crossbeam::channel::Sender;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use tracing::{error, info};

use crate::config::MyceliumConfig;
use crate::event::BusEvent;

/// watches a config file and sends ConfigReload events on changes.
pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    config: Arc<RwLock<MyceliumConfig>>,
}

impl ConfigWatcher {
    pub fn new(
        path: PathBuf,
        bus_sender: Sender<BusEvent>,
        config: Arc<RwLock<MyceliumConfig>>,
    ) -> Result<Self, crate::error::MyceliumError> {
        let config_clone = Arc::clone(&config);
        let path_clone = path.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| match res {
            Ok(event) => {
                if matches!(event.kind, EventKind::Modify(_)) {
                    match MyceliumConfig::load(&path_clone) {
                        Ok(new_config) => {
                            *config_clone.write() = new_config;
                            let _ = bus_sender.try_send(BusEvent::ConfigReload);
                            info!("Config reloaded");
                        }
                        Err(e) => {
                            error!("Config reload failed: {e}");
                        }
                    }
                }
            }
            Err(e) => error!("Config watch error: {e}"),
        })
        .map_err(|e| {
            crate::error::MyceliumError::Io(std::io::Error::other(e.to_string()))
        })?;

        watcher
            .watch(
                path.parent().unwrap_or(&path),
                RecursiveMode::NonRecursive,
            )
            .map_err(|e| {
                crate::error::MyceliumError::Io(std::io::Error::other(e.to_string()))
            })?;

        Ok(Self {
            _watcher: watcher,
            config,
        })
    }

    pub fn config(&self) -> Arc<RwLock<MyceliumConfig>> {
        Arc::clone(&self.config)
    }
}
