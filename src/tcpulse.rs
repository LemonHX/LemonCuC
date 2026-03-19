//! tcpulse constants (GStreamer audio pipeline command).

/// Default GStreamer pipeline command.
/// Captures from PulseAudio's monitor source (virtual_speaker.monitor),
/// encodes to AAC, muxes into fragmented MP4, and writes to stdout.
pub const DEFAULT_COMMAND: &str = "gst-launch-1.0 -q pulsesrc \
    device=virtual_speaker.monitor \
    ! audio/x-raw,channels=2,rate=24000 \
    ! voaacenc \
    ! mp4mux streamable=true fragment-duration=10 \
    ! fdsink fd=1";
