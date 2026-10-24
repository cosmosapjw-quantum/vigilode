//! Research-only dependency boundary for the exact pinned D3/D2 consumer.
//!
//! These modules are copied byte for byte from D3's published source snapshot.
//! Its inherited admission::environment reads MXCSR with unsafe assembly.
//! The new effects crate forbids unsafe code and links this dependency; no
//! production module or production unsafe policy is changed by this fixture.
#![crate_name = "d3_fixture"]

#[path = "reused/integer_gate.rs"]
pub mod integer_gate;
#[path = "reused/domain.rs"]
pub mod domain;
#[path = "reused/admission.rs"]
pub mod admission;
#[path = "reused/trajectory.rs"]
pub mod trajectory;
#[path = "reused/stage_safe.rs"]
pub mod stage_safe;
