mod common;

use common::*;
use hw_core::{Bank, Color, Player, Size, SystemId};
use hw_engine::{ActionKind, TurnError, TurnState, apply_action};

#[test]
fn catastrophe_purges_starless_homeworld_and_returns_remaining_ships() {
    let remaining_ship = ship(Player::One, Color::Blue, Size::Small);
    let state = state(
        vec![
            system(
                vec![
                    piece(Color::Red, Size::Small),
                    piece(Color::Red, Size::Medium),
                ],
                vec![
                    ship(Player::One, Color::Red, Size::Small),
                    ship(Player::Two, Color::Red, Size::Large),
                    remaining_ship,
                ],
            ),
            system(
                vec![piece(Color::Blue, Size::Large)],
                vec![ship(Player::Two, Color::Green, Size::Small)],
            ),
        ],
        [P1_HOME, P2_HOME],
    );

    let next = apply_action(&state, &catastrophe(P1_HOME, Color::Red)).expect("action applies");
    let homeworld = next.system(P1_HOME).expect("homeworld remains");

    assert_eq!(next.systems().len(), 2);
    assert!(homeworld.stars().is_empty());
    assert!(homeworld.ships().is_empty());
    assert_eq!(
        next.bank().count(Color::Blue, Size::Small),
        Bank::copies_per_piece() + 1
    );
    assert_eq!(
        next.bank().count(Color::Red, Size::Small),
        Bank::copies_per_piece() + 2
    );
    assert_eq!(
        next.bank().count(Color::Red, Size::Medium),
        Bank::copies_per_piece() + 1
    );
    assert_eq!(
        next.bank().count(Color::Red, Size::Large),
        Bank::copies_per_piece() + 1
    );
}

#[test]
fn catastrophe_prunes_starless_non_homeworlds_and_remaps_homeworld_ids() {
    let state = state(
        vec![
            system(
                vec![piece(Color::Yellow, Size::Small)],
                vec![ship(Player::One, Color::Green, Size::Small)],
            ),
            system(
                vec![
                    piece(Color::Red, Size::Small),
                    piece(Color::Red, Size::Medium),
                ],
                vec![
                    ship(Player::One, Color::Red, Size::Small),
                    ship(Player::Two, Color::Red, Size::Large),
                    ship(Player::One, Color::Blue, Size::Small),
                ],
            ),
            system(
                vec![piece(Color::Blue, Size::Large)],
                vec![ship(Player::Two, Color::Green, Size::Small)],
            ),
        ],
        [SystemId::new(0), SystemId::new(2)],
    );

    let next =
        apply_action(&state, &catastrophe(SystemId::new(1), Color::Red)).expect("action applies");

    assert_eq!(next.systems().len(), 2);
    assert_eq!(next.homeworld(Player::One), SystemId::new(0));
    assert_eq!(next.homeworld(Player::Two), SystemId::new(1));
    assert_eq!(
        next.bank().count(Color::Blue, Size::Small),
        Bank::copies_per_piece() + 1
    );
}

#[test]
fn sacrifice_turn_rejects_wrong_action_kind_but_still_allows_free_catastrophe() {
    let turn = TurnState::new(state_with_blue_sacrifice_and_catastrophe(), Player::One);

    let after_sacrifice = turn
        .apply_action(&sacrifice(
            Player::One,
            P1_HOME,
            ship(Player::One, Color::Blue, Size::Medium),
        ))
        .expect("sacrifice applies");

    assert_eq!(after_sacrifice.remaining_actions(), 2);
    assert_eq!(
        after_sacrifice.apply_action(&build(Player::One, P1_HOME, Color::Green, Size::Small,)),
        Err(TurnError::WrongSacrificeActionKind {
            expected: ActionKind::Trade,
            actual: ActionKind::Build,
        })
    );

    let after_catastrophe = after_sacrifice
        .apply_action(&catastrophe(P1_HOME, Color::Red))
        .expect("catastrophe applies");

    assert_eq!(after_catastrophe.remaining_actions(), 2);

    let after_trade = after_catastrophe
        .apply_action(&trade(
            Player::One,
            P1_HOME,
            ship(Player::One, Color::Blue, Size::Small),
            ship(Player::One, Color::Green, Size::Small),
        ))
        .expect("trade applies");

    assert_eq!(after_trade.remaining_actions(), 1);
}

fn state_with_blue_sacrifice_and_catastrophe() -> hw_core::GameState {
    state(
        vec![
            system(
                vec![
                    piece(Color::Red, Size::Small),
                    piece(Color::Green, Size::Medium),
                ],
                vec![
                    ship(Player::One, Color::Blue, Size::Medium),
                    ship(Player::One, Color::Blue, Size::Small),
                    ship(Player::One, Color::Red, Size::Small),
                    ship(Player::Two, Color::Red, Size::Medium),
                    ship(Player::Two, Color::Red, Size::Large),
                ],
            ),
            system(
                vec![piece(Color::Blue, Size::Large)],
                vec![ship(Player::Two, Color::Green, Size::Small)],
            ),
        ],
        [P1_HOME, P2_HOME],
    )
}
