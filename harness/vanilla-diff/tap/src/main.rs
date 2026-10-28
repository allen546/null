//! Vanilla-diff packet tap. Spec: specs/plans/VANILLA_DIFF_PLAN.md VD0.
//!
//! A transparent TCP proxy between a client and a server. Bytes are forwarded unchanged, in the
//! chunks they arrived in, so packet content and flush timing stay the client's. A copy of each
//! byte stream is decoded with `azalea-protocol` (offline mode: compression, no encryption) and
//! written as JSON lines: `{t_ms, tick, dir, name, summary}`. `tick` counts the serverbound
//! `ClientTickEnd` packets seen so far; the tick-end packet itself carries the tick it ends.
//!
//! Usage: `vd-tap [--listen 127.0.0.1:25569] [--target 127.0.0.1:25567] [--out tap.jsonl]`
//! Harness tool (scripts/check_boundary.sh): it may import Azalea and never runs in a bot.

use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use azalea::buf::AzBufVar;
use azalea::protocol::packets::{
    config::{ClientboundConfigPacket, ServerboundConfigPacket},
    game::{ClientboundGamePacket, ServerboundGamePacket},
    handshake::ServerboundHandshakePacket,
    login::{ClientboundLoginPacket, ServerboundLoginPacket},
};
use azalea::protocol::read::{compression_decoder, deserialize_packet};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone, Copy, PartialEq, Debug)]
enum State {
    Handshake,
    Status,
    Login,
    Config,
    Game,
}

/// Connection state shared by both directions: protocol state, compression, tick counter, log.
struct Shared {
    state: State,
    threshold: Option<u32>,
    tick: u64,
    start: Instant,
    out: Box<dyn Write + Send>,
}

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    Serverbound,
    Clientbound,
}

impl Shared {
    fn log(&mut self, dir: Dir, name: &str, summary: &str) {
        let line = serde_json::json!({
            "t_ms": self.start.elapsed().as_millis() as u64,
            "tick": self.tick,
            "state": format!("{:?}", self.state),
            "dir": if dir == Dir::Serverbound { "S" } else { "C" },
            "name": name,
            "summary": summary,
        });
        let _ = writeln!(self.out, "{line}");
        let _ = self.out.flush();
    }
}

/// Split a Debug string like `Name(Inner { .. })` into the variant name and a short summary.
fn name_and_summary(dbg: &str) -> (String, String) {
    let end = dbg.find(['(', ' ', '{']).unwrap_or(dbg.len());
    let name = dbg[..end].to_string();
    let mut summary: String = dbg[end..].chars().take(240).collect();
    if dbg.len() - end > summary.len() {
        summary.push_str("...");
    }
    (name, summary)
}

// Spec: specs/plans/VANILLA_DIFF_PLAN.md VD0
/// Decode one packet (frame payload after decompression) and update the shared state.
fn handle_packet(sh: &mut Shared, dir: Dir, body: &[u8]) {
    let mut cur = Cursor::new(body);
    macro_rules! decode {
        ($ty:ty) => {
            match deserialize_packet::<$ty>(&mut cur) {
                Ok(p) => Some(p),
                Err(e) => {
                    let mut c2 = Cursor::new(body);
                    let id = u32::azalea_read_var(&mut c2).unwrap_or(u32::MAX);
                    sh.log(dir, "DecodeError", &format!("id {id}, {} bytes: {e}", body.len()));
                    None
                }
            }
        };
    }
    macro_rules! emit {
        ($p:expr) => {{
            let (n, s) = name_and_summary(&format!("{:?}", $p));
            sh.log(dir, &n, &s);
        }};
    }
    match (sh.state, dir) {
        (State::Handshake, Dir::Serverbound) => {
            if let Some(p) = decode!(ServerboundHandshakePacket) {
                emit!(p);
                let ServerboundHandshakePacket::Intention(i) = p;
                sh.state = if i.intention as i32 == 1 { State::Status } else { State::Login };
            }
        }
        (State::Login, Dir::Serverbound) => {
            if let Some(p) = decode!(ServerboundLoginPacket) {
                emit!(p);
                if matches!(p, ServerboundLoginPacket::LoginAcknowledged(_)) {
                    sh.state = State::Config;
                }
            }
        }
        (State::Login, Dir::Clientbound) => {
            if let Some(p) = decode!(ClientboundLoginPacket) {
                emit!(p);
                if let ClientboundLoginPacket::LoginCompression(c) = &p {
                    sh.threshold = u32::try_from(c.compression_threshold).ok();
                }
            }
        }
        (State::Config, Dir::Serverbound) => {
            if let Some(p) = decode!(ServerboundConfigPacket) {
                emit!(p);
                if matches!(p, ServerboundConfigPacket::FinishConfiguration(_)) {
                    sh.state = State::Game;
                }
            }
        }
        (State::Config, Dir::Clientbound) => {
            if let Some(p) = decode!(ClientboundConfigPacket) {
                emit!(p);
            }
        }
        (State::Game, Dir::Serverbound) => {
            if let Some(p) = decode!(ServerboundGamePacket) {
                emit!(p);
                if matches!(p, ServerboundGamePacket::ClientTickEnd(_)) {
                    sh.tick += 1;
                }
            }
        }
        (State::Game, Dir::Clientbound) => {
            if let Some(p) = decode!(ClientboundGamePacket) {
                emit!(p);
            }
        }
        // Status pings and the clientbound handshake direction carry nothing we compare.
        _ => {}
    }
}

// Spec: specs/plans/VANILLA_DIFF_PLAN.md VD0
/// Pull every complete frame out of `buf`, decode it, and keep the partial tail.
fn feed(sh: &Arc<Mutex<Shared>>, dir: Dir, buf: &mut Vec<u8>) {
    loop {
        let mut cur = Cursor::new(&buf[..]);
        let Ok(len) = u32::azalea_read_var(&mut cur) else { return };
        let start = cur.position() as usize;
        let end = start + len as usize;
        if buf.len() < end {
            return;
        }
        let frame = buf[start..end].to_vec();
        buf.drain(..end);
        let mut sh = sh.lock().unwrap();
        let body = match sh.threshold {
            Some(t) => match compression_decoder(&mut Cursor::new(&frame[..]), t) {
                Ok(b) => b.into_vec(),
                Err(e) => {
                    sh.log(dir, "DecompressError", &e.to_string());
                    continue;
                }
            },
            None => frame,
        };
        handle_packet(&mut sh, dir, &body);
    }
}

// Spec: specs/plans/VANILLA_DIFF_PLAN.md VD0
/// Forward one direction, decoding a copy of the stream before each write so that the state
/// (compression threshold, tick count) is current when the other direction's reply is parsed.
async fn pump(
    mut from: tokio::net::tcp::OwnedReadHalf,
    mut to: tokio::net::tcp::OwnedWriteHalf,
    sh: Arc<Mutex<Shared>>,
    dir: Dir,
) {
    let mut chunk = vec![0u8; 64 * 1024];
    let mut pending: Vec<u8> = Vec::new();
    loop {
        let n = match from.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        pending.extend_from_slice(&chunk[..n]);
        feed(&sh, dir, &mut pending);
        if to.write_all(&chunk[..n]).await.is_err() {
            break;
        }
    }
    let _ = to.shutdown().await;
}

fn arg(name: &str, default: &str) -> String {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1).cloned()).unwrap_or(default.into())
}

// Spec: specs/plans/VANILLA_DIFF_PLAN.md VD0
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listen = arg("--listen", "127.0.0.1:25569");
    let target = arg("--target", "127.0.0.1:25567");
    let out_path = arg("--out", "tap.jsonl");
    let listener = TcpListener::bind(&listen).await?;
    eprintln!("vd-tap: {listen} -> {target}, log {out_path}");
    let mut n = 0;
    loop {
        let (client, _) = listener.accept().await?;
        n += 1;
        let path = if n == 1 { out_path.clone() } else { format!("{out_path}.{n}") };
        let server = TcpStream::connect(&target).await?;
        client.set_nodelay(true)?;
        server.set_nodelay(true)?;
        let sh = Arc::new(Mutex::new(Shared {
            state: State::Handshake,
            threshold: None,
            tick: 0,
            start: Instant::now(),
            out: Box::new(std::fs::File::create(path)?),
        }));
        let (cr, cw) = client.into_split();
        let (sr, sw) = server.into_split();
        tokio::spawn(pump(cr, sw, sh.clone(), Dir::Serverbound));
        tokio::spawn(pump(sr, cw, sh, Dir::Clientbound));
    }
}
