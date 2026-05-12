use hw_core::{GameState, Piece, Player, SystemId};

use super::{TransitionError, shared};

pub(super) fn apply(
    state: &GameState,
    _player: Player,
    system: SystemId,
    ship: Piece,
) -> Result<GameState, TransitionError> {
    let mut bank = state.bank().clone();
    shared::return_piece(&mut bank, ship)?;

    let (stars, mut ships) = shared::system_parts(state, system)?;
    shared::remove_ship(&mut ships, system, ship)?;

    let mut systems = state.systems().to_vec();
    systems[system.index()] = shared::rebuild_system(stars, ships)?;
    shared::rebuild_state_pruning(state, systems, bank, |_, system| system.ships().is_empty())
}
