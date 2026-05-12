use hw_core::{GameState, Piece, Player, SystemId};

use super::{TransitionError, shared};

pub(super) fn apply(
    state: &GameState,
    player: Player,
    system: SystemId,
    from: Piece,
    to: Piece,
) -> Result<GameState, TransitionError> {
    let mut bank = state.bank().clone();
    let drawn = shared::draw_piece(&mut bank, to)?.with_owner(player);
    shared::return_piece(&mut bank, from)?;

    let (stars, mut ships) = shared::system_parts(state, system)?;
    shared::remove_ship(&mut ships, system, from)?;
    ships.push(drawn);

    let mut systems = state.systems().to_vec();
    systems[system.index()] = shared::rebuild_system(stars, ships)?;
    shared::rebuild_state(state, systems, bank)
}
