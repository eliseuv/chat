//! Protocol
//! Internal messages between client and server
//!
//! This module defines the core message structures that form the logical
//! communication layer between the local server engine and connected clients.
//! These structures are high-level event representations, often containing detailed
//! metadata (like timestamps and addresses) for processing the message routing.

/// Message structures
pub mod message;
/// Request structures
pub mod request;
/// Response structures
pub mod response;
