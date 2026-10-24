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
    /// Id of the next command; a reply is only taken from packets that carry the command's id.
    next_id: i32,
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
        let mut r = Rcon { s, next_id: 2 };
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
    /// Vanilla splits a reply of 4096 bytes or more into several packets (RconClient#sendCmdResponse),
    /// all with the command's id. Packets with another id (left by an earlier reply) are dropped, so
    /// one long reply can never shift every later reply by one command.
    // Spec: specs/plans/ABILITY_CHECK.md "Which modules a change reruns"
    pub fn cmd(&mut self, c: &str) -> io::Result<String> {
        let started = std::time::Instant::now();
        let id = self.next_id;
        self.next_id = if id >= i32::MAX - 1 { 2 } else { id + 1 };
        self.s.write_all(&packet(id, 2, c))?;
        let reply = self.collect(id);
        WAIT_NS.fetch_add(started.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
        reply
    }

    fn collect(&mut self, id: i32) -> io::Result<String> {
        let mut text = String::new();
        loop {
            let (got, part) = self.read()?;
            if got != id {
                eprintln!("rcon: dropped a stale packet (id {got}, want {id}): {part:?}");
                continue;
            }
            let full = part.len() >= 4096;
            text.push_str(&part);
            if !full {
                return Ok(text);
            }
            break;
        }
        // A full packet may have a continuation; none follows when the reply was exactly 4096 long.
        // Only the wait for a packet's first byte is short: a timeout inside a packet would drop
        // the bytes already read and shift the stream again.
        loop {
            self.s.set_read_timeout(Some(Duration::from_millis(300)))?;
            let next = self.s.peek(&mut [0u8; 1]);
            self.s.set_read_timeout(Some(Duration::from_secs(30)))?;
            match next {
                Ok(0) => return Ok(text),
                Ok(_) => {}
                Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => return Ok(text),
                Err(e) => return Err(e),
            }
            let (got, part) = self.read()?;
            if got != id {
                eprintln!("rcon: dropped a stale packet (id {got}, want {id}): {part:?}");
                continue;
            }
            let full = part.len() >= 4096;
            text.push_str(&part);
            if !full {
                return Ok(text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// A server that answers the login, then a long reply in two packets, then a short one.
    /// The old reader took the first packet of the long reply and read the second one as the
    /// next command's reply (the p39_push desync).
    #[test]
    fn a_split_reply_does_not_shift_the_next_reply() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let t = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let mut buf = [0u8; 1460];
            c.read(&mut buf).unwrap();
            c.write_all(&packet(1, 2, "")).unwrap();
            c.read(&mut buf).unwrap();
            let id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
            c.write_all(&packet(id, 0, &"a".repeat(4096))).unwrap();
            c.write_all(&packet(id, 0, "tail")).unwrap();
            c.read(&mut buf).unwrap();
            let id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
            c.write_all(&packet(id, 0, "second")).unwrap();
        });
        let mut r = Rcon::connect(port, "pw").unwrap();
        let first = r.cmd("one").unwrap();
        assert_eq!(first.len(), 4100);
        assert!(first.ends_with("tail"));
        assert_eq!(r.cmd("two").unwrap(), "second");
        t.join().unwrap();
    }

    /// A packet left over from an earlier reply (other id) is never taken as this command's reply.
    #[test]
    fn a_stale_packet_is_dropped() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let t = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let mut buf = [0u8; 1460];
            c.read(&mut buf).unwrap();
            c.write_all(&packet(1, 2, "")).unwrap();
            c.read(&mut buf).unwrap();
            let id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
            c.write_all(&packet(id - 1, 0, "stale")).unwrap();
            c.write_all(&packet(id, 0, "fresh")).unwrap();
        });
        let mut r = Rcon::connect(port, "pw").unwrap();
        assert_eq!(r.cmd("x").unwrap(), "fresh");
        t.join().unwrap();
    }

    /// A continuation that stalls halfway (longer than the 300 ms wait) is still read whole, and a
    /// reply of exactly 4096 bytes ends after the wait with nothing lost from the next reply.
    #[test]
    fn a_stalled_continuation_and_an_exact_4096_reply_keep_the_stream_in_step() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let t = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let mut buf = [0u8; 1460];
            c.read(&mut buf).unwrap();
            c.write_all(&packet(1, 2, "")).unwrap();
            c.read(&mut buf).unwrap();
            let id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
            c.write_all(&packet(id, 0, &"a".repeat(4096))).unwrap();
            let tail = packet(id, 0, "tail");
            c.write_all(&tail[..6]).unwrap();
            std::thread::sleep(Duration::from_millis(600));
            c.write_all(&tail[6..]).unwrap();
            c.read(&mut buf).unwrap();
            let id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
            c.write_all(&packet(id, 0, &"b".repeat(4096))).unwrap();
            c.read(&mut buf).unwrap();
            let id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
            c.write_all(&packet(id, 0, "third")).unwrap();
        });
        let mut r = Rcon::connect(port, "pw").unwrap();
        let first = r.cmd("one").unwrap();
        assert_eq!(first.len(), 4100);
        assert!(first.ends_with("tail"));
        assert_eq!(r.cmd("two").unwrap(), "b".repeat(4096));
        assert_eq!(r.cmd("three").unwrap(), "third");
        t.join().unwrap();
    }
}
