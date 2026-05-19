#![doc = include_str!("../README.md")]

mod discourse;
mod entities;
mod language;
mod morphology;
mod pipeline;
mod syntax;
mod tokenization;

pub use discourse::*;
pub use entities::*;
pub use language::*;
pub use morphology::*;
pub use pipeline::*;
pub use syntax::*;
pub use tokenization::*;

#[cfg(test)]
mod tests;
