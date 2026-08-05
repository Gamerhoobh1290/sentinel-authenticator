//! Generated protobuf types for Google Authenticator migration payloads.
//!
//! This module just re-exports the prost-generated code from `gen/`.
//! The .proto schema lives at `proto/migration_payload.proto`.

pub mod gen;

pub use gen::migration::{MigrationPayload, OtpParameters};
