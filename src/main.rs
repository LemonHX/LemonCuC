//! LemonCUC backend – websockify + tcpulse in one binary.

mod tcpulse;
mod websockify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    tracing::info!("LemonCUC backend starting");

    // websockify #1: WS :6080 → VNC TCP :5900
    let ws_vnc = tokio::spawn({
        let ws_listen = "0.0.0.0:6080";
        let vnc_target = "127.0.0.1:5900";
        async move {
            if let Err(e) = websockify::run(ws_listen, vnc_target).await {
                tracing::error!("websockify(vnc) error: {e}");
            }
        }
    });

    // tcpulse: raw TCP audio server on internal port :5701
    let audio_tcp = tokio::spawn({
        let audio_listen = "127.0.0.1:5701";
        async move {
            if let Err(e) = tcpulse::run(audio_listen).await {
                tracing::error!("tcpulse error: {e}");
            }
        }
    });

    // websockify #2: WS :5702 → tcpulse TCP :5701
    let ws_audio = tokio::spawn({
        let ws_listen = "0.0.0.0:5702";
        let audio_target = "127.0.0.1:5701";
        async move {
            if let Err(e) = websockify::run(ws_listen, audio_target).await {
                tracing::error!("websockify(audio) error: {e}");
            }
        }
    });

    tokio::select! {
        r = ws_vnc => { r?; }
        r = audio_tcp => { r?; }
        r = ws_audio => { r?; }
    }

    Ok(())
}
