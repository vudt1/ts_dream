//! Golden acceptance harness (Chapter 9).
//!
//! Ships a golden-file parser and a runner that connects a socket to the Rust
//! server, sends the `<<` C2S frames in order, and diffs the `>>` S2C frames
//! byte-exact against the golden. Capture proxy (ticket 05) is provided as a
//! reusable helper in `proxy`.

use crate::error::{Result, TsError};
use crate::protocol::frame;

/// Protocol constants for the harness (Chapter 9 §9.7).
pub const XOR_KEY: u8 = 0xAD;

/// One golden scenario: directed frames parsed from a `.golden` text file.
#[derive(Debug, Default)]
pub struct Golden {
    pub name: String,
    pub c2s: Vec<String>,
    pub s2c: Vec<String>,
}

impl Golden {
    /// Parse a golden file. Format:
    /// - `// comment`
    /// - `<<HEX`     client → server frame
    /// - `>>HEX`     server → client frame
    /// - blank line groups (ignored for diffing)
    pub fn parse(text: &str, name: &str) -> Result<Self> {
        let mut g = Golden {
            name: name.to_string(),
            ..Default::default()
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("<<") {
                g.c2s.push(rest.trim().to_uppercase());
            } else if let Some(rest) = line.strip_prefix(">>") {
                g.s2c.push(rest.trim().to_uppercase());
            } else {
                return Err(TsError::Protocol(format!(
                    "golden {}: bad line: {}",
                    name, line
                )));
            }
        }
        Ok(g)
    }
}

/// The runner: feed every C2S frame to the server socket, collect S2C frames,
/// and compare against the golden S2C stream frame-by-frame byte-exact.
///
/// Returns the sorted S2C received, or the first mismatch.
pub async fn run_golden(
    golden: &Golden,
    addr: &str,
) -> Result<Vec<String>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let mut sock = TcpStream::connect(addr)
        .await
        .map_err(|e| TsError::Io(e))?;

    // Send each C2S frame (hex → xor → write).
    for c2s in &golden.c2s {
        let wire = frame::encode_to_wire(c2s)?;
        sock.write_all(&wire)
            .await
            .map_err(|e| TsError::Io(e))?;
    }

    // Read back until close; collect S2C frames.
    let mut decoder = frame::Decoder::new();
    let mut buffer = vec![0u8; 8192];
    let mut s2c: Vec<String> = Vec::new();
    loop {
        let n = sock.read(&mut buffer).await.map_err(|e| TsError::Io(e))?;
        if n == 0 {
            break;
        }
        // NOTE: buffer here is raw wire bytes → decoder.feed XORs it back.
        s2c.extend(decoder.feed(&buffer[..n]));
    }

    if s2c == golden.s2c {
        Ok(s2c)
    } else {
        Err(TsError::Other(format!(
            "golden `{}`: S2C mismatch (expected {} frames, got {})",
            golden.name,
            golden.s2c.len(),
            s2c.len()
        )))
    }
}