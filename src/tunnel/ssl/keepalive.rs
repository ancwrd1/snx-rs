use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::{channel::oneshot, SinkExt};
use tracing::{trace, warn};

use crate::{
    model::proto::KeepaliveRequestData,
    platform::{self},
    tunnel::ssl::PacketSender,
};

const KEEPALIVE_MAX_RETRIES: u64 = 3;
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

pub struct KeepaliveRunner {
    interval: Duration,
    sender: PacketSender,
    keepalive_counter: Arc<AtomicU64>,
}

impl KeepaliveRunner {
    pub fn new(interval: Duration, sender: PacketSender, counter: Arc<AtomicU64>) -> Self {
        Self {
            interval,
            sender,
            keepalive_counter: counter,
        }
    }

    pub async fn run(&self) {
        let (stop_sender, stop_receiver) = oneshot::channel();

        let interval = self.interval;
        let keepalive_counter = self.keepalive_counter.clone();
        let mut sender = self.sender.clone();

        tokio::spawn(async move {
            loop {
                if platform::is_online() {
                    if keepalive_counter.load(Ordering::SeqCst) >= KEEPALIVE_MAX_RETRIES {
                        let msg = "No response for keepalive packets, tunnel appears stuck";
                        warn!(msg);
                        break;
                    }

                    let req = KeepaliveRequestData { id: "0".to_string() };
                    trace!("Keepalive request: {:?}", req);

                    keepalive_counter.fetch_add(1, Ordering::SeqCst);

                    match tokio::time::timeout(SEND_TIMEOUT, sender.send(req.into())).await {
                        Ok(Ok(_)) => {}
                        _ => {
                            warn!("Cannot send keepalive packet, exiting");
                            break;
                        }
                    }
                }
                tokio::time::sleep(interval).await;
            }
            let _ = stop_sender.send(());
        });

        let _ = stop_receiver.await;
    }
}
