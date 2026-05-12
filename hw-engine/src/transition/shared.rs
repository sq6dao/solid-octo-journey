use hw_core::{Bank, GameState, Piece, Player, StarSystem, SystemId};

use crate::action::ActionError;

use super::TransitionError;

pub(super) fn system_parts(
    state: &GameState,
    system: SystemId,
) -> Result<(Vec<Piece>, Vec<Piece>), TransitionError> {
    let system_ref =
        state
            .system(system)
            .ok_or(TransitionError::InvalidAction(ActionError::UnknownSystem {
                system,
            }))?;

    Ok((system_ref.stars().to_vec(), system_ref.ships().to_vec()))
}

pub(super) fn rebuild_system(
    stars: Vec<Piece>,
    ships: Vec<Piece>,
) -> Result<StarSystem, TransitionError> {
    StarSystem::new(stars, ships).map_err(TransitionError::from)
}

pub(super) fn rebuild_state(
    state: &GameState,
    systems: Vec<StarSystem>,
    bank: Bank,
) -> Result<GameState, TransitionError> {
    GameState::new(systems, homeworlds(state), bank).map_err(TransitionError::from)
}

pub(super) fn homeworlds(state: &GameState) -> [SystemId; Player::COUNT] {
    [state.homeworld(Player::One), state.homeworld(Player::Two)]
}

pub(super) fn draw_piece(bank: &mut Bank, piece: Piece) -> Result<Piece, TransitionError> {
    bank.draw(piece.color(), piece.size())
        .map_err(|_| TransitionError::InvalidAction(ActionError::PieceUnavailable { piece }))
}

pub(super) fn remove_ship(
    ships: &mut Vec<Piece>,
    system: SystemId,
    ship: Piece,
) -> Result<Piece, TransitionError> {
    let index = ships
        .iter()
        .position(|candidate| *candidate == ship)
        .ok_or(TransitionError::InvalidAction(
            ActionError::ShipNotPresent { system, ship },
        ))?;

    Ok(ships.remove(index))
}

pub(super) fn rebuild_state_pruning<F>(
    state: &GameState,
    systems: Vec<StarSystem>,
    mut bank: Bank,
    should_prune: F,
) -> Result<GameState, TransitionError>
where
    F: Fn(SystemId, &StarSystem) -> bool,
{
    let homeworlds = homeworlds(state);
    let mut remapped = vec![None; systems.len()];
    let mut kept_systems = Vec::new();

    for (index, system) in systems.into_iter().enumerate() {
        let id = SystemId::new(index);
        let is_homeworld = homeworlds.contains(&id);

        if !is_homeworld && should_prune(id, &system) {
            return_system_pieces(&mut bank, &system)?;
        } else {
            remapped[index] = Some(SystemId::new(kept_systems.len()));
            kept_systems.push(system);
        }
    }

    let homeworlds = [
        remap_homeworld(homeworlds[0], &remapped)?,
        remap_homeworld(homeworlds[1], &remapped)?,
    ];

    GameState::new(kept_systems, homeworlds, bank).map_err(TransitionError::from)
}

fn remap_homeworld(
    homeworld: SystemId,
    remapped: &[Option<SystemId>],
) -> Result<SystemId, TransitionError> {
    remapped
        .get(homeworld.index())
        .and_then(|id| *id)
        .ok_or(TransitionError::InvalidAction(ActionError::UnknownSystem {
            system: homeworld,
        }))
}

fn return_system_pieces(bank: &mut Bank, system: &StarSystem) -> Result<(), TransitionError> {
    for piece in system.stars().iter().chain(system.ships()) {
        return_piece(bank, *piece)?;
    }

    Ok(())
}

pub(super) fn return_piece(bank: &mut Bank, piece: Piece) -> Result<(), TransitionError> {
    let unowned = Piece::new(piece.color(), piece.size());
    bank.return_piece(unowned)
        .map_err(|_| TransitionError::InvalidAction(ActionError::UnownedShip { ship: piece }))
}
