#![doc = include_str!("../README.md")]

mod analyzers;
mod bundles;
mod conformance;
mod download;
mod external;
mod predictions;
mod presets;
mod spec;

pub use analyzers::*;
pub use bundles::*;
pub use conformance::*;
pub use download::*;
pub use external::*;
pub use predictions::*;
pub use presets::*;
pub use spec::*;

#[cfg(test)]
mod tests;
