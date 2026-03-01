//! Import adapters for URI-schemed imports.
//!
//! Each adapter handles a specific URI scheme and produces
//! runtime JavaScript to inject the external data source.

pub mod shadertoy;
pub mod midi;
pub mod osc;
pub mod camera;

/// The kind of import source identified by URI scheme.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportScheme {
    /// Local file import (no scheme prefix)
    File,
    /// `shadertoy://XXXX` — import from Shadertoy
    Shadertoy(String),
    /// `midi://channel/N` — MIDI controller input
    Midi { channel: u8 },
    /// `osc://host:port/path` — OSC over WebSocket
    Osc { host: String, port: u16, path: String },
    /// `camera://N` — webcam texture source
    Camera { device_index: u32 },
}

/// Parse a URI string into an ImportScheme.
pub fn parse_uri(uri: &str) -> ImportScheme {
    if let Some(id) = uri.strip_prefix("shadertoy://") {
        ImportScheme::Shadertoy(id.to_string())
    } else if let Some(rest) = uri.strip_prefix("midi://") {
        let channel = rest.strip_prefix("channel/")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        ImportScheme::Midi { channel }
    } else if let Some(rest) = uri.strip_prefix("osc://") {
        // osc://host:port/path
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        let (host, port) = if let Some(hp) = parts.first() {
            let hp_parts: Vec<&str> = hp.splitn(2, ':').collect();
            (
                hp_parts.first().unwrap_or(&"localhost").to_string(),
                hp_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(9000),
            )
        } else {
            ("localhost".to_string(), 9000)
        };
        let path = parts.get(1).unwrap_or(&"").to_string();
        ImportScheme::Osc { host, port, path }
    } else if let Some(rest) = uri.strip_prefix("camera://") {
        let device_index = rest.parse().unwrap_or(0);
        ImportScheme::Camera { device_index }
    } else {
        ImportScheme::File
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shadertoy_uri() {
        assert_eq!(
            parse_uri("shadertoy://XsXXDn"),
            ImportScheme::Shadertoy("XsXXDn".into())
        );
    }

    #[test]
    fn parse_midi_uri() {
        assert_eq!(
            parse_uri("midi://channel/1"),
            ImportScheme::Midi { channel: 1 }
        );
    }

    #[test]
    fn parse_osc_uri() {
        match parse_uri("osc://localhost:9000/params") {
            ImportScheme::Osc { host, port, path } => {
                assert_eq!(host, "localhost");
                assert_eq!(port, 9000);
                assert_eq!(path, "params");
            }
            other => panic!("expected Osc, got {other:?}"),
        }
    }

    #[test]
    fn parse_camera_uri() {
        assert_eq!(
            parse_uri("camera://0"),
            ImportScheme::Camera { device_index: 0 }
        );
    }

    #[test]
    fn parse_file_uri() {
        assert_eq!(parse_uri("lib/base.game"), ImportScheme::File);
    }
}
