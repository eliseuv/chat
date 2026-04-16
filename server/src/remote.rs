//! Remote
//! Messages between local client thread and remote client
//!
//! This module handles the physical wire-level protocol. It defines the exact
//! structures serialized into binary, dispatched over the network via length
//! delimited frames, and explicitly decoded on the other side.
//!
//! It also provides the `RemotePacketCodec`, which implements `tokio_util::codec::Encoder`
//! and `tokio_util::codec::Decoder` for asynchronous, non-blocking stream framing via CBOR.

/// Codec structures
pub mod codec;
/// Packet structures
pub mod packet;
