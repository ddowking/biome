//! Encapsulates logic required for updating the Biome Supervisor
//! itself.

use crate::{env,
            util};
use biome_common::{command::package::install::InstallSource,
                     ui::UI};
use biome_core::{package::{PackageIdent,
                             PackageInstall},
                   ChannelIdent};
use std::{thread,
          time::Duration};
use time::{Duration as TimeDuration,
           SteadyTime};
use tokio::{self,
            runtime,
            sync::oneshot::{self,
                            error::TryRecvError,
                            Receiver,
                            Sender},
            time as tokiotime};

pub const SUP_PKG_IDENT: &str = "biome/bio-sup";
const DEFAULT_FREQUENCY: i64 = 60_000;
const FREQUENCY_ENVVAR: &str = "HAB_SUP_UPDATE_MS";

pub struct SelfUpdater {
    rx:             Receiver<PackageInstall>,
    current:        PackageIdent,
    update_url:     String,
    update_channel: ChannelIdent,
}

// TODO (CM): Want to use the Periodic trait here, but can't due to
// how things are currently structured (The service updater had a worker)

impl SelfUpdater {
    pub fn new(current: PackageIdent, update_url: String, update_channel: ChannelIdent) -> Self {
        let rx = Self::init(current.clone(), update_url.clone(), update_channel.clone());
        SelfUpdater { rx,
                      current,
                      update_url,
                      update_channel }
    }

    /// Spawn a new Supervisor updater thread.
    fn init(current: PackageIdent,
            update_url: String,
            update_channel: ChannelIdent)
            -> Receiver<PackageInstall> {
        let (tx, rx) = oneshot::channel();
        // Execute this future on a dedicated thread, and using a
        // single-threaded runtime. Eventually, this should use `tokio::spawn`,
        // but that will require refactoring to make the future safe
        // to spawn on an executor.
        //
        // (Note: the `enable_all` is to ensure the Tokio timer is set
        // up on the runtime, which is needed both here and also in
        // the async reqwest calls we make from here.)
        thread::Builder::new().name("self-updater".to_string())
                              .spawn(move || {
                                  let mut rt =
                                      runtime::Builder::new().basic_scheduler()
                                                             .enable_all()
                                                             .build()
                                                             .expect("Could not spawn runtime \
                                                                      for self-updater thread!");

                                  rt.block_on(Self::run(tx, &current, &update_url, &update_channel))
                              })
                              .expect("Unable to start self-updater thread");
        rx
    }

    async fn run(tx: Sender<PackageInstall>,
                 current: &PackageIdent,
                 update_url: &str,
                 update_channel: &ChannelIdent) {
        debug!("Self updater current package, {}", current);
        // SUP_PKG_IDENT will always parse as a valid PackageIdent,
        // and thus a valid InstallSource
        let install_source: InstallSource = SUP_PKG_IDENT.parse().unwrap();
        loop {
            let next_check = SteadyTime::now() + TimeDuration::milliseconds(update_frequency());

            match util::pkg::install(// We don't want anything in here to print
                                     &mut UI::with_sinks(),
                                     &update_url,
                                     &install_source,
                                     &update_channel).await
            {
                Ok(package) => {
                    if current < package.ident() {
                        debug!("Self updater installing newer Supervisor, {}",
                               package.ident());
                        tx.send(package).expect("Main thread has gone away!");
                        break;
                    } else {
                        debug!("Supervisor package found is not newer than ours");
                    }
                }
                Err(err) => {
                    warn!("Self updater failed to get latest, {}", err);
                }
            }

            let time_to_wait = (next_check - SteadyTime::now()).num_milliseconds();
            if time_to_wait > 0 {
                tokiotime::delay_for(Duration::from_millis(time_to_wait as u64)).await;
            }
        }
    }

    pub async fn updated(&mut self) -> Option<PackageInstall> {
        match self.rx.try_recv() {
            Ok(package) => Some(package),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Closed) => {
                debug!("Self updater has died, restarting...");
                self.rx = Self::init(self.current.clone(),
                                     self.update_url.clone(),
                                     self.update_channel.clone());
                None
            }
        }
    }
}

fn update_frequency() -> i64 {
    match env::var(FREQUENCY_ENVVAR) {
        Ok(val) => val.parse::<i64>().unwrap_or(DEFAULT_FREQUENCY),
        Err(_) => DEFAULT_FREQUENCY,
    }
}
