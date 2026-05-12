use hw_core::{GameState, Piece, Player, SystemId};

use super::{TransitionError, shared};

pub(super) fn apply(
    state: &GameState,
    player: Player,
    system: SystemId,
    target: Piece,
) -> Result<GameState, TransitionError> {
    let bank = state.bank().clone();
    let (stars, mut ships) = shared::system_parts(state, system)?;
    shared::remove_ship(&mut ships, system, target)?;
    ships.push(target.with_owner(player));

    let mut systems = state.systems().to_vec();
    systems[system.index()] = shared::rebuild_system(stars, ships)?;
    shared::rebuild_state(state, systems, bank)
}
