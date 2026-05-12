use hw_core::{GameState, Piece, Player, StarSystem, SystemId};

use crate::action::MoveTarget;

use super::{TransitionError, shared};

pub(super) fn apply(
    state: &GameState,
    _player: Player,
    from: SystemId,
    ship: Piece,
    target: &MoveTarget,
) -> Result<GameState, TransitionError> {
    match target {
        MoveTarget::Existing(to) => apply_existing(state, from, ship, *to),
        MoveTarget::New { stars } => apply_new(state, from, ship, stars),
    }
}

fn apply_existing(
    state: &GameState,
    from: SystemId,
    ship: Piece,
    to: SystemId,
) -> Result<GameState, TransitionError> {
    let bank = state.bank().clone();
    let mut systems = state.systems().to_vec();
    let (from_stars, mut from_ships) = shared::system_parts(state, from)?;
    let moved_ship = shared::remove_ship(&mut from_ships, from, ship)?;
    systems[from.index()] = shared::rebuild_system(from_stars, from_ships)?;

    let (to_stars, mut to_ships) = shared::system_parts(state, to)?;
    to_ships.push(moved_ship);
    systems[to.index()] = shared::rebuild_system(to_stars, to_ships)?;

    shared::rebuild_state_pruning(state, systems, bank, |_, system| system.ships().is_empty())
}

fn apply_new(
    state: &GameState,
    from: SystemId,
    ship: Piece,
    stars: &[Piece],
) -> Result<GameState, TransitionError> {
    let mut bank = state.bank().clone();
    let mut systems = state.systems().to_vec();
    let (from_stars, mut from_ships) = shared::system_parts(state, from)?;
    let moved_ship = shared::remove_ship(&mut from_ships, from, ship)?;
    systems[from.index()] = shared::rebuild_system(from_stars, from_ships)?;

    let mut drawn_stars = Vec::new();
    for star in stars {
        drawn_stars.push(shared::draw_piece(&mut bank, *star)?);
    }
    systems.push(StarSystem::new(drawn_stars, vec![moved_ship])?);

    shared::rebuild_state_pruning(state, systems, bank, |_, system| system.ships().is_empty())
}
