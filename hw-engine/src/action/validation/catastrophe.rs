use hw_core::{Color, GameState, StarSystem, SystemId};

use super::ActionError;

pub(super) fn has_possible(state: &GameState) -> bool {
    state.systems().iter().any(|system| {
        Color::ALL
            .into_iter()
            .any(|color| catastrophe_count(system, color) >= 4)
    })
}

pub(super) fn validate(
    state: &GameState,
    system: SystemId,
    color: Color,
) -> Result<(), ActionError> {
    let system_ref = state
        .system(system)
        .ok_or(ActionError::UnknownSystem { system })?;
    let count = catastrophe_count(system_ref, color);

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

fn catastrophe_count(system: &StarSystem, color: Color) -> usize {
    system
        .stars()
        .iter()
        .chain(system.ships())
        .filter(|piece| piece.color() == color)
        .count()
}
