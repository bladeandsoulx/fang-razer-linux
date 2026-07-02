//! Shared protocol definitions for Fang.
//!
//! - [`packet`]: builds the 91-byte USB HID feature reports understood by the
//!   embedded controller on Razer Blade laptops. Byte layout and command ids
//!   verified against razer-laptop-control-no-dkms (GPL-2.0), which is the
//!   known-working open implementation of this protocol.
//! - [`api`]: the JSON-lines request/response/event types spoken between
//!   `fangd` and its clients over the local socket.
//! - [`models`]: table of supported laptops keyed by USB product id.

pub mod api;
pub mod models;
pub mod packet;
