//! E1-bench: a matched-accuracy work–precision harness that runs VigilODE
//! against external production comparators (Diffsol, SUNDIALS CVODE, Netlib
//! DVODE, Hairer RADAU5 and RODAS) on one shared model per problem, and
//! generates its Markdown report from the results JSON only (plan §7).
#![warn(missing_docs)]

pub mod alloc;
pub mod arms;
pub mod corpus;
pub mod ffi;
pub mod harness;
pub mod metrics;
pub mod model;
pub mod reference_policy;
pub mod report;
pub mod source_identity;
