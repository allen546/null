//! Minimal RCON client (the protocol of harness/skeleton-server/scripts/rcon.py).

use std::{
    io::{self, Read, Write},
    net::TcpStream,
    time::Duration,
};

/// Total time spent waiting for RCON replies, in nanoseconds (the timing summary reads it).
pub static WAIT_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct Rcon {
    s: TcpStream,
}

fn packet(id: i32, ty: i32, body: &str) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend((10 + body.len() as i32).to_le_bytes());
    b.extend(id.to_le_bytes());
    b.extend(ty.to_le_bytes());
    b.extend(body.as_bytes());
    b.extend([0, 0]);
    b
}

impl Rcon {
    pub fn connect(port: u16, password: &str) -> io::Result<Rcon> {
        let s = TcpStream::connect(("127.0.0.1", port))?;
        s.set_read_timeout(Some(Duration::from_secs(30)))?;
        s.set_nodelay(true)?;
        let mut r = Rcon { s };
        r.s.write_all(&packet(1, 3, password))?;
        let (id, _) = r.read()?;
        if id == -1 {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "rcon: wrong password"));
        }
        Ok(r)
    }

    fn read(&mut self) -> io::Result<(i32, String)> {
        let mut len = [0u8; 4];
        self.s.read_exact(&mut len)?;
        let mut body = vec![0u8; i32::from_le_bytes(len).max(10) as usize];
        self.s.read_exact(&mut body)?;
        let id = i32::from_le_bytes(body[0..4].try_into().unwrap());
        let n = body.len() - 2;
        Ok((id, String::from_utf8_lossy(&body[8..n]).into_owned()))
    }

    /// Runs one command and returns the server's reply text.
    pub fn cmd(&mut self, c: &str) -> io::Result<String> {
        let started = std::time::Instant::now();
        self.s.write_all(&packet(2, 2, c))?;
        let reply = self.read();
        WAIT_NS.fetch_add(started.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
        Ok(reply?.1)
    }
}
