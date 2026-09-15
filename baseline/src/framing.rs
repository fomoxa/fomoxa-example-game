use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

pub const KIND_HELLO: u8 = 1;
pub const KIND_WELCOME: u8 = 2;
pub const KIND_INPUT: u8 = 3;
pub const KIND_SNAPSHOT: u8 = 4;

pub const HEADER_LEN: usize = 5;
pub const RECV_BUFFER: usize = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum SendState {
    Sent,
    Congested,
    Closed,
}

pub fn frame(kind: u8, payload: &[u8], out: &mut Vec<u8>) {
    out.clear();
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.push(kind);
    out.extend_from_slice(payload);
}

#[derive(Default)]
struct Outbox {
    bytes: Vec<u8>,
    offset: usize,
}

pub struct Link {
    stream: TcpStream,
    inbox: Vec<u8>,
    outbox: Outbox,
    scratch: Vec<u8>,
    closed: bool,
}

impl Link {
    pub fn connect<A: ToSocketAddrs>(address: A) -> io::Result<Link> {
        Link::adopt(TcpStream::connect(address)?)
    }

    pub fn adopt(stream: TcpStream) -> io::Result<Link> {
        stream.set_nonblocking(true)?;
        let _ = stream.set_nodelay(true);
        Ok(Link {
            stream,
            inbox: Vec::with_capacity(RECV_BUFFER),
            outbox: Outbox::default(),
            scratch: Vec::with_capacity(RECV_BUFFER),
            closed: false,
        })
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn peer_label(&self) -> String {
        self.stream
            .peer_addr()
            .map_or_else(|_| "unknown address".to_owned(), |address| address.to_string())
    }

    pub fn send(&mut self, kind: u8, payload: &[u8]) -> SendState {
        if self.closed {
            return SendState::Closed;
        }
        self.flush();
        if !self.pending().is_empty() {
            return SendState::Congested;
        }

        frame(kind, payload, &mut self.scratch);
        let mut written = 0;
        while written < self.scratch.len() {
            match self.stream.write(&self.scratch[written..]) {
                Ok(0) => {
                    self.closed = true;
                    return SendState::Closed;
                }
                Ok(count) => written += count,
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.closed = true;
                    return SendState::Closed;
                }
            }
        }
        if written < self.scratch.len() {
            self.outbox.bytes.clear();
            self.outbox.bytes.extend_from_slice(&self.scratch[written..]);
            self.outbox.offset = 0;
        }
        SendState::Sent
    }

    pub fn flush(&mut self) {
        while !self.pending().is_empty() {
            let pending = self.outbox.offset;
            match self.stream.write(&self.outbox.bytes[pending..]) {
                Ok(0) => {
                    self.closed = true;
                    return;
                }
                Ok(count) => self.outbox.offset += count,
                Err(error) if error.kind() == ErrorKind::WouldBlock => return,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.closed = true;
                    return;
                }
            }
        }
        self.outbox.bytes.clear();
        self.outbox.offset = 0;
    }

    fn pending(&self) -> &[u8] {
        &self.outbox.bytes[self.outbox.offset.min(self.outbox.bytes.len())..]
    }

    pub fn poll(&mut self, buffer: &mut [u8], mut handler: impl FnMut(u8, &[u8])) {
        if self.closed {
            return;
        }
        self.flush();
        loop {
            match self.stream.read(buffer) {
                Ok(0) => {
                    self.closed = true;
                    break;
                }
                Ok(count) => {
                    self.inbox.extend_from_slice(&buffer[..count]);
                    if count < buffer.len() {
                        break;
                    }
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.closed = true;
                    break;
                }
            }
        }

        let mut offset = 0;
        while self.inbox.len() - offset >= HEADER_LEN {
            let length = u32::from_le_bytes([
                self.inbox[offset],
                self.inbox[offset + 1],
                self.inbox[offset + 2],
                self.inbox[offset + 3],
            ]) as usize;
            if self.inbox.len() - offset - HEADER_LEN < length {
                break;
            }
            let kind = self.inbox[offset + 4];
            let start = offset + HEADER_LEN;
            handler(kind, &self.inbox[start..start + length]);
            offset = start + length;
        }
        if offset > 0 {
            self.inbox.drain(..offset);
        }
    }
}
