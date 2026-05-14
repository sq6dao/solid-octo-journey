mod types;
mod validation;

pub use types::{Action, ActionKind, TravelTarget};
pub use validation::{ActionError, validate_action};

#[cfg(test)]
mod tests;
