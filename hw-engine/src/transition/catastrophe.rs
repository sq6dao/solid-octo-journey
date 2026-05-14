use hw_core::{Color, GameState, Piece, SystemId};

use super::{TransitionError, shared};

pub(super) fn apply(
    state: &GameState,
    system: SystemId,
    color: Color,
) -> Result<GameState, TransitionError> {
    let mut bank = state.bank().clone();
    let (stars, ships) = shared::system_parts(state, system)?;
    let stars = remove_color(&mut bank, stars, color)?;
    let mut ships = remove_color(&mut bank, ships, color)?;

    if stars.is_empty() && shared::homeworlds(state).contains(&system) {
        return_pieces(&mut bank, ships)?;
        ships = Vec::new();
    }

    let mut systems = state.systems().to_vec();
    systems[system.index()] = shared::rebuild_system(stars, ships)?;
    shared::rebuild_state_pruning(state, systems, bank, |id, candidate| {
        id == system && (candidate.ships().is_empty() || candidate.stars().is_empty())
    })
}

fn remove_color(
    bank: &mut hw_core::Bank,
    pieces: Vec<Piece>,
    color: Color,
) -> Result<Vec<Piece>, TransitionError> {
    let mut kept = Vec::new();

    for piece in pieces {
        if piece.color() == color {
            shared::return_piece(bank, piece)?;
        } else {
            kept.push(piece);
        }
    }

    Ok(kept)
}

fn return_pieces(
    bank: &mut hw_core::Bank,
    pieces: Vec<Piece>,
) -> Result<(), TransitionError> {
    for piece in pieces {
        shared::return_piece(bank, piece)?;
    }

    Ok(())
}
