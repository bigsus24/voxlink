//! # ChatCall Core
//!
//! Room management, chat, user profiles, and local storage.
//! Orchestrates `chatcall-net` and `chatcall-audio` into a
//! complete communication system.

pub mod room;
pub mod chat;
pub mod user;
pub mod storage;
pub mod events;
