mod types;
mod validation;

pub use types::{Action, ActionKind, TravelTarget};
pub use validation::{ActionError, has_possible_catastrophe, validate_action};

#[cfg(test)]
mod tests;
