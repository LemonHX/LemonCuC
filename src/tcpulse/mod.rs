//! TCP audio streaming server (tcpulse).
//!
//! Listens for TCP connections. When a client connects, spawns a
//! GStreamer pipeline and streams its stdout to the client.
//! This replaces the C `tcpulse` program.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::process::Command;

/// Default GStreamer pipeline command.
/// Captures from PulseAudio's monitor source (virtual_speaker.monitor),
/// encodes to AAC, muxes into fragmented MP4, and writes to stdout.
pub const DEFAULT_COMMAND: &str = "gst-launch-1.0 -q pulsesrc \
    device=virtual_speaker.monitor \
    ! audio/x-raw,channels=2,rate=24000 \
    ! voaacenc \
    ! mp4mux streamable=true fragment-duration=10 \
    ! fdsink fd=1";

/// Run the tcpulse audio streaming server.
///
/// * `listen_addr` – e.g. `"0.0.0.0:5702"`
pub async fn run(listen_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(listen_addr).await?;
    let cmd = DEFAULT_COMMAND.to_owned();
    tracing::info!("tcpulse listening on {listen_addr}");

    loop {
        let (stream, peer) = listener.accept().await?;
        let cmd = cmd.clone();
        tokio::spawn(async move {
            tracing::info!("tcpulse: client connected from {peer}");
            if let Err(e) = handle_client(stream, &cmd).await {
                tracing::warn!("tcpulse: client {peer} ended: {e}");
            }
        });
    }
}

/// Handle a single audio-streaming client.
async fn handle_client(
    mut stream: tokio::net::TcpStream,
    command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Spawn the pipeline process.
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    let mut stdout = child.stdout.take().ok_or("failed to open child stdout")?;

    let mut buf = vec![0u8; 4096];
    let mut discard = vec![0u8; 1024];

    loop {
        tokio::select! {
            // Read from the pipeline stdout.
            result = stdout.read(&mut buf) => {
                match result {
                    Ok(0) | Err(_) => {
                        tracing::debug!("tcpulse: pipeline ended");
                        break;
                    }
                    Ok(n) => {
                        if stream.write_all(&buf[..n]).await.is_err() {
                            tracing::debug!("tcpulse: client disconnected");
                            break;
                        }
                    }
                }
            }
            // Also try to read (and discard) any data the client sends.
            result = stream.read(&mut discard) => {
                match result {
                    Ok(0) | Err(_) => {
                        tracing::debug!("tcpulse: client disconnected");
                        break;
                    }
                    _ => {} // discard
                }
            }
        }
    }

    // Kill the child in case it's still running.
    let _ = child.kill().await;
    let _ = stream.shutdown().await;

    Ok(())
}
