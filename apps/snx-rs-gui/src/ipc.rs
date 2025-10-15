use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use interprocess::local_socket::{
    GenericNamespaced, Name, ToNsName,
    traits::tokio::{Listener, Stream},
};
use tokio::sync::mpsc::Sender;
use tokio_util::codec::{Decoder, LengthDelimitedCodec};
use tracing::debug;

use crate::tray::TrayEvent;

pub async fn send_event(event: TrayEvent) -> anyhow::Result<()> {
    debug!("Sending event: {:?}", event);
    let stream = interprocess::local_socket::tokio::Stream::connect(ipc_name()?).await?;
    let mut codec = LengthDelimitedCodec::new().framed(stream);
    codec.send(Bytes::copy_from_slice(event.as_str().as_bytes())).await?;
    Ok(())
}

pub fn start_ipc_listener(sender: Sender<TrayEvent>) -> anyhow::Result<()> {
    let listener = interprocess::local_socket::ListenerOptions::new()
        .name(ipc_name()?)
        .create_tokio()?;

    tokio::spawn(async move {
        debug!("Started IPC listener");
        while let Ok(conn) = listener.accept().await {
            debug!("Accepted IPC connection");
            let sender = sender.clone();
            tokio::spawn(async move { handle_connection(conn, sender).await });
        }
    });

    Ok(())
}

fn ipc_name() -> anyhow::Result<Name<'static>> {
    let uid = unsafe { libc::getuid() };
    Ok(format!("snx-rs-gui-{uid}").to_ns_name::<GenericNamespaced>()?)
}

async fn handle_connection(
    stream: interprocess::local_socket::tokio::Stream,
    sender: Sender<TrayEvent>,
) -> anyhow::Result<()> {
    let mut codec = LengthDelimitedCodec::new().framed(stream);
    while let Some(Ok(event)) = codec.next().await {
        if let Ok(event) = String::from_utf8_lossy(&event).parse::<TrayEvent>() {
            debug!("Received event: {:?}", event);
            sender.send(event).await?;
        }
    }
    Ok(())
}
