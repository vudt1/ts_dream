//! Modern 3NF persistence layer (ticket 06).
//!
//! Layout:
//! - [`model`]: storage selectors and row value objects (ThingData-backed).
//! - [`traits`]: the async repository traits domain code depends on.
//! - [`sqlite`]: pool-backed implementations of those traits.
//! - [`transactions`]: atomic money/item operations (trade, shop, bank).

pub mod model;
pub mod sqlite;
pub mod traits;
pub mod transactions;
