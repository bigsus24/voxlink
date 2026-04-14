use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::protocol::codec::PacketCodec;
use crate::protocol::packet::PacketError;
use std::net::SocketAddr;

/// Error type for TCP channel operations
#[derive(Debug, thiserror::Error)]
pub enum TcpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Packet error: {0}")]
    Packet(#[from] PacketError),

    #[error("Connection closed by peer")]
    ConnectionClosed,

    #[error("Send buffer full")]
    BufferFull,
}

/// TCP channel for reliable control and chat communication.
///
/// Wraps a tokio `TcpStream` with the ChatCall packet codec for
/// length-prefixed framing. All data sent/received is properly framed
/// so multiple packets can be distinguished on the byte stream.
pub struct TcpChannel {
    stream: TcpStream,
    codec: PacketCodec,
    /// Read buffer for raw TCP bytes
    read_buf: Vec<u8>,
    /// Remote peer address
    peer_addr: SocketAddr,
}

impl TcpChannel {
    /// Buffer size for TCP reads
    const READ_BUF_SIZE: usize = 8192;

    /// Wrap an existing TCP stream
    pub fn new(stream: TcpStream) -> Result<Self, TcpError> {
        let peer_addr = stream.peer_addr()?;
        Ok(Self {
            stream,
            codec: PacketCodec::new(),
            read_buf: vec![0u8; Self::READ_BUF_SIZE],
            peer_addr,
        })
    }

    /// Get the remote peer's address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Send a raw packet (already serialized to bytes) over TCP.
    /// The packet should include the PacketHeader.
    pub async fn send_packet(&mut self, packet_bytes: &[u8]) -> Result<(), TcpError> {
        let framed = PacketCodec::encode(packet_bytes);
        self.stream.write_all(&framed).await?;
        self.stream.flush().await?;
        Ok(())
    }

    /// Receive the next complete packet from the TCP stream.
    /// Blocks until a complete packet is available or the connection closes.
    /// Returns the full packet bytes (header + payload).
    pub async fn recv_packet(&mut self) -> Result<Vec<u8>, TcpError> {
        loop {
            // First try to decode from buffered data
            if let Some(result) = self.codec.try_decode() {
                return result.map_err(TcpError::Packet);
            }

            // Need more data from the socket
            let n = self.stream.read(&mut self.read_buf).await?;
            if n == 0 {
                return Err(TcpError::ConnectionClosed);
            }
            self.codec.feed(&self.read_buf[..n]);
        }
    }

    /// Send a packet and immediately flush
    pub async fn send_and_flush(&mut self, packet_bytes: &[u8]) -> Result<(), TcpError> {
        self.send_packet(packet_bytes).await
    }

    /// Shutdown the TCP connection gracefully
    pub async fn shutdown(&mut self) -> Result<(), TcpError> {
        self.stream.shutdown().await?;
        Ok(())
    }

    /// Check if there are buffered bytes waiting to be decoded
    pub fn has_buffered_data(&self) -> bool {
        self.codec.buffered_len() > 0
    }

    /// Get a mutable reference to the underlying stream (for advanced use)
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }
}
