//! Golden acceptance harness (Chapter 9).
//!
//! Ships a golden-file parser and a runner that connects a socket to the Rust
//! server, sends the `<<` C2S frames in order, and diffs the `>>` S2C frames
//! byte-exact against the golden. Capture proxy (ticket 05) is provided as a
//! reusable helper in `proxy`.

use crate::error::{Result, TsError};
use crate::protocol::frame;
use std::path::Path;

/// Protocol constants for the harness (Chapter 9 §9.7 / Chapter 8 §8.2).
pub const XOR_KEY: u8 = 0xAD;
pub const FRAME_MAGIC: &[u8] = &[0xF4, 0x44];
pub const ID_PREFIX: &str = "vn";
pub const MIN_VERSION: u16 = 186;
pub const SERVER_NAME: &str = "TSVN";

/// One golden scenario: directed frames parsed from a `.golden` text file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

    /// Load a golden scenario from a `.golden` file path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path_ref = path.as_ref();
        let name = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let content = std::fs::read_to_string(path_ref).map_err(TsError::Io)?;
        Self::parse(&content, name)
    }

    /// Serialize the golden scenario into formatted text.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("// Golden scenario: ");
        out.push_str(&self.name);
        out.push('\n');
        for c in &self.c2s {
            out.push_str("<<");
            out.push_str(c);
            out.push('\n');
        }
        if !self.c2s.is_empty() && !self.s2c.is_empty() {
            out.push('\n');
        }
        for s in &self.s2c {
            out.push_str(">>");
            out.push_str(s);
            out.push('\n');
        }
        out
    }

    /// Save the golden scenario to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        std::fs::write(path, self.to_text()).map_err(TsError::Io)
    }

    /// Load all `.golden` files from a directory, sorted by name.
    pub fn load_dir(dir_path: impl AsRef<Path>) -> Result<Vec<Self>> {
        let mut goldens = Vec::new();
        let entries = match std::fs::read_dir(dir_path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(goldens),
            Err(e) => return Err(TsError::Io(e)),
        };

        for entry in entries {
            let entry = entry.map_err(TsError::Io)?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "golden" {
                        let g = Self::from_file(&path)?;
                        goldens.push(g);
                    }
                }
            }
        }
        goldens.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(goldens)
    }
}

/// The runner: feed every C2S frame to the server socket, collect S2C frames,
/// and compare against the golden S2C stream frame-by-frame byte-exact.
///
/// Returns the S2C received on success, or the first mismatch error.
pub async fn run_golden(
    golden: &Golden,
    addr: &str,
) -> Result<Vec<String>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let mut sock = TcpStream::connect(addr)
        .await
        .map_err(TsError::Io)?;

    // Send each C2S frame (hex → xor → write).
    for c2s in &golden.c2s {
        let wire = frame::encode_to_wire(c2s)?;
        sock.write_all(&wire)
            .await
            .map_err(TsError::Io)?;
    }

    // Read back until expected S2C count is met or timeout/EOF.
    let mut decoder = frame::Decoder::new();
    let mut buffer = vec![0u8; 8192];
    let mut s2c: Vec<String> = Vec::new();
    let expected_count = golden.s2c.len();

    while s2c.len() < expected_count {
        let read_res = timeout(Duration::from_millis(500), sock.read(&mut buffer)).await;
        match read_res {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                s2c.extend(decoder.feed(&buffer[..n]));
            }
            Ok(Err(e)) => return Err(TsError::Io(e)),
            Err(_) => break, // Timeout waiting for further frames
        }
    }

    if s2c == golden.s2c {
        Ok(s2c)
    } else {
        Err(TsError::Other(format!(
            "golden `{}`: S2C mismatch (expected {} frames: {:?}, got {} frames: {:?})",
            golden.name,
            golden.s2c.len(),
            golden.s2c,
            s2c.len(),
            s2c
        )))
    }
}

/// Capture proxy module (Chapter 9 §9.2).
pub mod proxy {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    /// Direction of a captured frame.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Direction {
        C2S,
        S2C,
    }

    /// Capture proxy TCP listener that forwards traffic between game client and server,
    /// logging all frames in XOR-decoded hex format.
    pub struct CaptureProxy {
        listen_addr: String,
        target_addr: String,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl CaptureProxy {
        pub fn new(listen_addr: impl Into<String>, target_addr: impl Into<String>) -> Self {
            Self {
                listen_addr: listen_addr.into(),
                target_addr: target_addr.into(),
                log: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn log_buffer(&self) -> Arc<Mutex<Vec<String>>> {
            Arc::clone(&self.log)
        }

        pub fn captured_lines(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }

        pub fn to_golden_text(&self, comment: &str) -> String {
            let mut out = String::new();
            if !comment.is_empty() {
                out.push_str("// ");
                out.push_str(comment);
                out.push('\n');
            }
            for line in self.captured_lines() {
                out.push_str(&line);
                out.push('\n');
            }
            out
        }

        /// Run the proxy accept loop.
        pub async fn run(&self, mut shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Result<()> {
            let listener = TcpListener::bind(&self.listen_addr)
                .await
                .map_err(TsError::Io)?;

            loop {
                tokio::select! {
                    res = listener.accept() => {
                        let (client_stream, _) = res.map_err(TsError::Io)?;
                        let target_addr = self.target_addr.clone();
                        let log = Arc::clone(&self.log);
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(client_stream, &target_addr, log).await {
                                tracing::debug!("Proxy session ended: {:?}", e);
                            }
                        });
                    }
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                }
            }
            Ok(())
        }
    }

    async fn handle_connection(
        client: TcpStream,
        target_addr: &str,
        log: Arc<Mutex<Vec<String>>>,
    ) -> Result<()> {
        let target = TcpStream::connect(target_addr).await.map_err(TsError::Io)?;
        let (mut c_read, mut c_write) = client.into_split();
        let (mut t_read, mut t_write) = target.into_split();

        let log_c2s = Arc::clone(&log);
        let log_s2c = Arc::clone(&log);

        let c2s_task = tokio::spawn(async move {
            let mut decoder = frame::Decoder::new();
            let mut buf = vec![0u8; 8192];
            loop {
                let n = c_read.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                let raw = &buf[..n];
                let frames = decoder.feed(raw);
                {
                    let mut guard = log_c2s.lock().unwrap();
                    for f in frames {
                        guard.push(format!("<<{f}"));
                    }
                }
                t_write.write_all(raw).await?;
            }
            Ok::<(), std::io::Error>(())
        });

        let s2c_task = tokio::spawn(async move {
            let mut decoder = frame::Decoder::new();
            let mut buf = vec![0u8; 8192];
            loop {
                let n = t_read.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                let raw = &buf[..n];
                let frames = decoder.feed(raw);
                {
                    let mut guard = log_s2c.lock().unwrap();
                    for f in frames {
                        guard.push(format!(">>{f}"));
                    }
                }
                c_write.write_all(raw).await?;
            }
            Ok::<(), std::io::Error>(())
        });

        let _ = tokio::try_join!(c2s_task, s2c_task);
        Ok(())
    }
}

/// Deterministic, in-process golden scenario (Ch9 §9.4/§9.5).
///
/// A scenario is the replayable unit the golden files lock: it seeds a fresh
/// `Conn` via `setup`, feeds the ordered `c2s` frames through the opcode
/// dispatcher (the same path a live connection takes), and yields the ordered
/// server→client frame stream. Because every input is a pure function of the
/// seeded session + `GameData`, the replay is byte-deterministic and needs no
/// socket, DB, or wall clock.
pub mod scenario {
    use super::*;
    use crate::battle::service::BattleService;
    use crate::data::loader::GameData;
    use crate::protocol::encoder;
    use crate::server::handler;
    use crate::server::session::Conn;
    use std::path::Path;

    /// One replayable golden scenario.
    pub struct Scenario<'a> {
        pub name: String,
        /// Seeds the fresh connection's session state before replay.
        pub setup: Box<dyn Fn(&mut Conn) + Send + Sync + 'a>,
        /// Static data tables the dispatcher reads.
        pub data: GameData,
        /// Ordered client→server frames (uppercase hex, `F444…`).
        pub c2s: Vec<String>,
        /// Fixed unix seconds for the `Thoi gian` banner (deterministic replay,
        /// Ch9 §9.2). `None` = real wall clock.
        pub now_override: Option<i64>,
    }

    impl<'a> Scenario<'a> {
        pub fn new(
            name: impl Into<String>,
            data: GameData,
            c2s: Vec<String>,
            setup: impl Fn(&mut Conn) + Send + Sync + 'a,
        ) -> Self {
            Self {
                name: name.into(),
                setup: Box::new(setup),
                data,
                c2s,
                now_override: None,
            }
        }

        /// Pin the time banner to a fixed instant so the replay is
        /// wall-clock independent.
        pub fn with_now(mut self, unix_secs: i64) -> Self {
            self.now_override = Some(unix_secs);
            self
        }

        /// Replay the scenario in-process and return the ordered S2C frames.
        pub async fn replay(&self) -> Vec<String> {
            if let Some(t) = self.now_override {
                crate::server::spawn::override_now(t);
            }
            let mut conn = Conn::new();
            (self.setup)(&mut conn);
            let service = BattleService::new(std::sync::Arc::new(self.data.clone()));
            let mut s2c: Vec<String> = Vec::new();
            for c2s in &self.c2s {
                let Some(decoded) = encoder::bytes(c2s) else {
                    panic!("scenario `{}`: bad c2s hex `{c2s}`", self.name);
                };
                let out = handler::dispatch(
                    &mut conn,
                    &decoded,
                    &self.data,
                    &service,
                    &handler::ServerEnv::none(),
                )
                .await;
                for frame in out.outgoing {
                    s2c.push(frame);
                }
            }
            if self.now_override.is_some() {
                crate::server::spawn::reset_now();
            }
            s2c
        }

        /// Serialize the replayed scenario into golden-file text.
        pub async fn to_golden_text(&self, comment: &str) -> String {
            let mut out = String::new();
            out.push_str("// ");
            out.push_str(comment);
            out.push('\n');
            for c in &self.c2s {
                out.push_str("<<");
                out.push_str(c);
                out.push('\n');
            }
            if !self.c2s.is_empty() {
                out.push('\n');
            }
            let s2c = self.replay().await;
            for s in s2c {
                out.push_str(">>");
                out.push_str(&s);
                out.push('\n');
            }
            out
        }

        /// Save the replayed golden text to `golden/<name>.golden`.
        pub async fn save(&self, dir: impl AsRef<Path>, comment: &str) -> Result<()> {
            let path = dir.as_ref().join(format!("{}.golden", self.name));
            std::fs::write(&path, self.to_golden_text(comment).await).map_err(TsError::Io)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::watch;

    #[tokio::test]
    async fn test_golden_serialization() -> Result<()> {
        let text = "// Sample scenario\n<<F444010000\n>>F4440300010901\n";
        let g = Golden::parse(text, "sample")?;
        assert_eq!(g.c2s, vec!["F444010000"]);
        assert_eq!(g.s2c, vec!["F4440300010901"]);
        let serialized = g.to_text();
        assert!(serialized.contains("<<F444010000"));
        assert!(serialized.contains(">>F4440300010901"));
        Ok(())
    }

    #[tokio::test]
    async fn test_capture_proxy_forwarding() -> Result<()> {
        // 1. Start a mock server
        let mock_listener = TcpListener::bind("127.0.0.1:0").await.map_err(TsError::Io)?;
        let server_addr = mock_listener.local_addr().map_err(TsError::Io)?;

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = mock_listener.accept().await {
                let mut buf = vec![0u8; 1024];
                if let Ok(n) = socket.read(&mut buf).await {
                    let req_wire = &buf[..n];
                    // Verify received C2S wire packet (XOR of F444010000)
                    let decoded = req_wire.iter().map(|b| b ^ XOR_KEY).collect::<Vec<_>>();
                    assert_eq!(decoded, vec![0xF4, 0x44, 0x01, 0x00, 0x00]);

                    // Send response S2C wire packet for F4440300010901
                    let resp_hex = "F4440300010901";
                    let resp_wire = frame::encode_to_wire(resp_hex).unwrap();
                    let _ = socket.write_all(&resp_wire).await;
                }
            }
        });

        // 2. Start proxy
        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.map_err(TsError::Io)?;
        let proxy_addr = proxy_listener.local_addr().map_err(TsError::Io)?;
        drop(proxy_listener); // release port for CaptureProxy

        let proxy = Arc::new(proxy::CaptureProxy::new(proxy_addr.to_string(), server_addr.to_string()));
        let (tx, rx) = watch::channel(false);
        let proxy_clone = Arc::clone(&proxy);
        let proxy_task = tokio::spawn(async move {
            let _ = proxy_clone.run(rx).await;
        });

        // Wait a tiny bit for proxy to start listening
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // 3. Client connects to proxy and sends F444010000 wire bytes
        let mut client = tokio::net::TcpStream::connect(proxy_addr).await.map_err(TsError::Io)?;
        let req_wire = frame::encode_to_wire("F444010000")?;
        client.write_all(&req_wire).await.map_err(TsError::Io)?;

        let mut resp_buf = vec![0u8; 1024];
        let n = client.read(&mut resp_buf).await.map_err(TsError::Io)?;
        let resp_decoded = resp_buf[..n].iter().map(|b| b ^ XOR_KEY).collect::<Vec<_>>();
        assert_eq!(resp_decoded, vec![0xF4, 0x44, 0x03, 0x00, 0x01, 0x09, 0x01]);

        let _ = tx.send(true);
        let _ = proxy_task.await;

        // 4. Verify captured lines in proxy
        let lines = proxy.captured_lines();
        assert!(lines.contains(&"<<F444010000".to_string()));
        assert!(lines.contains(&">>F4440300010901".to_string()));

        Ok(())
    }
}