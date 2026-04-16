use futures::io;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use tokio_util::{
    bytes::Bytes,
    codec::{Decoder, Encoder, LengthDelimitedCodec},
};
use super::packet::{IncomingPacket, OutgoingPacket};

/// A bipartite serialization/deserialization codec for framing TCP streams.
///
/// This codec transforms raw asynchronous byte streams into discrete application-level 
/// generic packets utilizing CBOR (Concise Binary Object Representation) format under the hood.
/// The framing is managed by an underlying `LengthDelimitedCodec` which safely chunks the bytes.
///
/// Generic Parameters:
/// - `In`: The structure type expected to be decoded from incoming bytes.
/// - `Out`: The structure type provided to be encoded into outgoing bytes.
pub struct RemotePacketCodec<In, Out> {
    /// Handles appending and parsing frame length boundaries over the TCP buffer.
    framer: LengthDelimitedCodec,
    /// Resolves the generic `In` and `Out` markers at compile time without occupying memory.
    _phantom: PhantomData<(In, Out)>,
}

impl<In, Out> RemotePacketCodec<In, Out> {
    /// Creates a fresh codec engine equipped with a default length-delimited framer.
    pub fn new() -> Self {
        Self {
            framer: LengthDelimitedCodec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<In, Out> Default for RemotePacketCodec<In, Out> {
    fn default() -> Self {
        Self::new()
    }
}

impl<In, Out> Encoder<Out> for RemotePacketCodec<In, Out>
where
    Out: Serialize,
{
    type Error = io::Error;

    /// Converts a strongly-typed `Out` struct into bytes, then prefixes it with a length frame.
    fn encode(
        &mut self,
        item: Out,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        // Serialize the application packet into a temporary memory buffer using CBOR format.
        let mut cbor_buffer = Vec::new();
        ciborium::into_writer(&item, &mut cbor_buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        // Frame the exact length of the serialized data and append it to the outgoing stream.
        self.framer.encode(Bytes::from(cbor_buffer), dst)?;

        Ok(())
    }
}

impl<In, Out> Decoder for RemotePacketCodec<In, Out>
where
    In: DeserializeOwned,
{
    type Item = In;

    type Error = io::Error;

    /// Attempts to read enough bytes from `src` to complete a frame and deserialize it into `In`.
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        // Extract a single complete frame from the stream using the length prefix.
        // Returns `None` if the frame is strictly incomplete (waiting for more TCP segments).
        let frame = match self.framer.decode(src)? {
            None => return Ok(None), // Not enough data yet
            Some(frame) => frame,
        };

        // Complete frame acquired. Parse the pure inner structure synchronously from memory.
        match ciborium::from_reader(frame.as_ref()) {
            Ok(packet) => Ok(Some(packet)),
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
        }
    }
}

/// The designated codec specialization utilized by the Server.
/// 
/// - **Decodes**: `IncomingPacket` (Client claims traversing upwards)
/// - **Encodes**: `OutgoingPacket` (Server notifications pushing downwards)
pub type ServerCodec = RemotePacketCodec<IncomingPacket, OutgoingPacket>;

/// The designated codec specialization utilized by the Client.
///
/// - **Decodes**: `OutgoingPacket` (Server notifications traversing downwards)
/// - **Encodes**: `IncomingPacket` (Client claims pushing upwards)
pub type ClientCodec = RemotePacketCodec<OutgoingPacket, IncomingPacket>;
