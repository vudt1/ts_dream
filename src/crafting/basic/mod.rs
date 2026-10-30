//! Basic compounding module using `Compound.Dat`.

pub mod algorithm;
pub mod catalog;
pub mod validator;

pub use algorithm::{delphi_round, execute_compound, CraftOutput, DelphiRng};
pub use catalog::CompoundCatalog;
pub use validator::{validate_compound_inputs, CraftInputSlot};
