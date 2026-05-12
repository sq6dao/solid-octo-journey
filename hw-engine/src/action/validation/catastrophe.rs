use hw_core::{Color, GameState, SystemId};

use super::ActionError;

pub(super) fn validate(
    state: &GameState,
    system: SystemId,
    color: Color,
) -> Result<(), ActionError> {
    let system_ref = state
        .system(system)
        .ok_or(ActionError::UnknownSystem { system })?;
    let count = system_ref
        .stars()
        .iter()
        .chain(system_ref.ships())
        .filter(|piece| piece.color() == color)
        .count();

    if count >= 4 {
        Ok(())
    } else {
        Err(ActionError::NoCatastrophe {
            system,
            color,
            count,
        })
    }
}
