use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, SystemId};

use crate::{Action, ActionError, TransitionError, apply_action};

#[test]
fn apply_build_action_adds_the_ship_and_draws_from_the_bank() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Green, Size::Small);
    let action = Action::Build {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    let next = apply_action(&state, &action).expect("action applies");

    assert_eq!(
        count_ship(next.system(SystemId::new(0)).expect("system exists"), ship),
        2
    );
    assert_eq!(
        next.bank().count(Color::Green, Size::Small),
        Bank::copies_per_piece() - 1
    );
    assert_eq!(
        state.bank().count(Color::Green, Size::Small),
        Bank::copies_per_piece()
    );
}

#[test]
fn apply_build_action_returns_validation_errors() {
    let state = state_with_empty_bank_piece(Color::Green, Size::Small);
    let ship = owned_ship(Player::One, Color::Green, Size::Small);
    let action = Action::Build {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    assert_eq!(
        apply_action(&state, &action),
        Err(TransitionError::InvalidAction(
            ActionError::PieceUnavailable { piece: ship }
        ))
    );
}

fn valid_state() -> GameState {
    state_with_bank(Bank::new())
}

fn state_with_empty_bank_piece(color: Color, size: Size) -> GameState {
    let mut bank = Bank::new();
    for _ in 0..Bank::copies_per_piece() {
        bank.draw(color, size).expect("piece exists");
    }

    state_with_bank(bank)
}

fn state_with_bank(bank: Bank) -> GameState {
    GameState::new(
        vec![
            StarSystem::new(
                vec![Piece::new(Color::Yellow, Size::Small)],
                vec![
                    owned_ship(Player::One, Color::Blue, Size::Small),
                    owned_ship(Player::One, Color::Green, Size::Small),
                    owned_ship(Player::One, Color::Yellow, Size::Small),
                    owned_ship(Player::Two, Color::Red, Size::Medium),
                ],
            )
            .expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Green, Size::Medium)],
                vec![owned_ship(Player::Two, Color::Yellow, Size::Small)],
            )
            .expect("system is valid"),
        ],
        [SystemId::new(0), SystemId::new(1)],
        bank,
    )
    .expect("state is valid")
}

fn count_ship(system: &StarSystem, ship: Piece) -> usize {
    system
        .ships()
        .iter()
        .filter(|candidate| **candidate == ship)
        .count()
}

fn owned_ship(player: Player, color: Color, size: Size) -> Piece {
    Piece::owned(color, size, player)
}
