use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, SystemId};

use crate::{Action, ActionError, MoveTarget, TransitionError, apply_action};

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

#[test]
fn apply_move_action_moves_a_ship_between_existing_systems() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Blue, Size::Small);
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: MoveTarget::Existing(SystemId::new(1)),
    };

    let next = apply_action(&state, &action).expect("action applies");

    assert_eq!(
        count_ship(next.system(SystemId::new(0)).expect("system exists"), ship),
        0
    );
    assert_eq!(
        count_ship(next.system(SystemId::new(1)).expect("system exists"), ship),
        1
    );
    assert_eq!(
        next.bank().count(Color::Blue, Size::Small),
        state.bank().count(Color::Blue, Size::Small)
    );
}

#[test]
fn apply_move_action_discovers_a_new_system_and_draws_stars() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Blue, Size::Small);
    let star = Piece::new(Color::Red, Size::Medium);
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: MoveTarget::New { stars: vec![star] },
    };

    let next = apply_action(&state, &action).expect("action applies");
    let discovered = next.system(SystemId::new(2)).expect("system exists");

    assert_eq!(next.systems().len(), 3);
    assert_eq!(discovered.stars(), &[star]);
    assert_eq!(discovered.ships(), &[ship]);
    assert_eq!(
        next.bank().count(Color::Red, Size::Medium),
        Bank::copies_per_piece() - 1
    );
}

#[test]
fn apply_move_action_prunes_an_empty_non_homeworld_source() {
    let source_star = Piece::new(Color::Blue, Size::Medium);
    let ship = owned_ship(Player::One, Color::Yellow, Size::Small);
    let state = state_with_systems_and_homeworlds(
        Bank::new(),
        vec![
            StarSystem::new(
                vec![Piece::new(Color::Yellow, Size::Small)],
                vec![owned_ship(Player::One, Color::Green, Size::Small)],
            )
            .expect("system is valid"),
            StarSystem::new(vec![source_star], vec![ship]).expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Red, Size::Large)],
                vec![owned_ship(Player::Two, Color::Red, Size::Medium)],
            )
            .expect("system is valid"),
        ],
        [SystemId::new(0), SystemId::new(2)],
    );
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(1),
        ship,
        target: MoveTarget::Existing(SystemId::new(0)),
    };

    let next = apply_action(&state, &action).expect("action applies");

    assert_eq!(next.systems().len(), 2);
    assert_eq!(next.homeworld(Player::Two), SystemId::new(1));
    assert_eq!(
        count_ship(next.system(SystemId::new(0)).expect("system exists"), ship),
        1
    );
    assert_eq!(
        next.bank().count(Color::Blue, Size::Medium),
        Bank::copies_per_piece() + 1
    );
}

#[test]
fn apply_move_action_keeps_an_empty_homeworld_source() {
    let ship = owned_ship(Player::One, Color::Yellow, Size::Small);
    let state = state_with_systems_and_homeworlds(
        Bank::new(),
        vec![
            StarSystem::new(vec![Piece::new(Color::Blue, Size::Medium)], vec![ship])
                .expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Red, Size::Large)],
                vec![owned_ship(Player::Two, Color::Red, Size::Medium)],
            )
            .expect("system is valid"),
        ],
        [SystemId::new(0), SystemId::new(1)],
    );
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: MoveTarget::Existing(SystemId::new(1)),
    };

    let next = apply_action(&state, &action).expect("action applies");

    assert_eq!(next.systems().len(), 2);
    assert!(
        next.system(SystemId::new(0))
            .expect("system exists")
            .ships()
            .is_empty()
    );
    assert_eq!(next.homeworld(Player::One), SystemId::new(0));
}

#[test]
fn apply_trade_action_replaces_a_ship_and_updates_the_bank() {
    let state = valid_state();
    let from = owned_ship(Player::One, Color::Blue, Size::Small);
    let to = owned_ship(Player::One, Color::Red, Size::Small);
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from,
        to,
    };

    let next = apply_action(&state, &action).expect("action applies");
    let system = next.system(SystemId::new(0)).expect("system exists");

    assert_eq!(count_ship(system, from), 0);
    assert_eq!(count_ship(system, to), 1);
    assert_eq!(
        next.bank().count(Color::Red, Size::Small),
        Bank::copies_per_piece() - 1
    );
    assert_eq!(
        next.bank().count(Color::Blue, Size::Small),
        Bank::copies_per_piece() + 1
    );
}

#[test]
fn apply_trade_action_returns_validation_errors() {
    let state = valid_state();
    let from = owned_ship(Player::One, Color::Blue, Size::Small);
    let to = owned_ship(Player::One, Color::Red, Size::Medium);
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from,
        to,
    };

    assert_eq!(
        apply_action(&state, &action),
        Err(TransitionError::InvalidAction(ActionError::SizeMismatch {
            from,
            to,
        }))
    );
}

#[test]
fn apply_invade_action_changes_the_target_owner() {
    let state = valid_state();
    let target = owned_ship(Player::One, Color::Blue, Size::Small);
    let captured = owned_ship(Player::Two, Color::Blue, Size::Small);
    let action = Action::Invade {
        player: Player::Two,
        system: SystemId::new(0),
        target,
    };

    let next = apply_action(&state, &action).expect("action applies");
    let system = next.system(SystemId::new(0)).expect("system exists");

    assert_eq!(count_ship(system, target), 0);
    assert_eq!(count_ship(system, captured), 1);
    assert_eq!(
        next.bank().count(Color::Blue, Size::Small),
        state.bank().count(Color::Blue, Size::Small)
    );
}

#[test]
fn apply_invade_action_returns_validation_errors() {
    let target = owned_ship(Player::Two, Color::Blue, Size::Medium);
    let state = state_with_primary_ships(vec![
        owned_ship(Player::One, Color::Red, Size::Small),
        target,
    ]);
    let action = Action::Invade {
        player: Player::One,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(
        apply_action(&state, &action),
        Err(TransitionError::InvalidAction(
            ActionError::CannotInvadeLargerShip {
                player: Player::One,
                target,
            }
        ))
    );
}

#[test]
fn apply_sacrifice_action_removes_the_ship_and_returns_it_to_the_bank() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Blue, Size::Small);
    let action = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    let next = apply_action(&state, &action).expect("action applies");

    assert_eq!(
        count_ship(next.system(SystemId::new(0)).expect("system exists"), ship),
        0
    );
    assert_eq!(
        next.bank().count(Color::Blue, Size::Small),
        Bank::copies_per_piece() + 1
    );
}

#[test]
fn apply_sacrifice_action_prunes_an_empty_non_homeworld_system() {
    let star = Piece::new(Color::Blue, Size::Medium);
    let ship = owned_ship(Player::One, Color::Blue, Size::Small);
    let state = state_with_systems_and_homeworlds(
        Bank::new(),
        vec![
            StarSystem::new(
                vec![Piece::new(Color::Yellow, Size::Small)],
                vec![owned_ship(Player::One, Color::Green, Size::Small)],
            )
            .expect("system is valid"),
            StarSystem::new(vec![star], vec![ship]).expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Red, Size::Large)],
                vec![owned_ship(Player::Two, Color::Red, Size::Medium)],
            )
            .expect("system is valid"),
        ],
        [SystemId::new(0), SystemId::new(2)],
    );
    let action = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(1),
        ship,
    };

    let next = apply_action(&state, &action).expect("action applies");

    assert_eq!(next.systems().len(), 2);
    assert_eq!(next.homeworld(Player::Two), SystemId::new(1));
    assert_eq!(
        next.bank().count(Color::Blue, Size::Small),
        Bank::copies_per_piece() + 1
    );
    assert_eq!(
        next.bank().count(Color::Blue, Size::Medium),
        Bank::copies_per_piece() + 1
    );
}

#[test]
fn apply_sacrifice_action_keeps_an_empty_homeworld() {
    let ship = owned_ship(Player::One, Color::Blue, Size::Small);
    let state = state_with_systems_and_homeworlds(
        Bank::new(),
        vec![
            StarSystem::new(vec![Piece::new(Color::Yellow, Size::Small)], vec![ship])
                .expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Red, Size::Large)],
                vec![owned_ship(Player::Two, Color::Red, Size::Medium)],
            )
            .expect("system is valid"),
        ],
        [SystemId::new(0), SystemId::new(1)],
    );
    let action = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    let next = apply_action(&state, &action).expect("action applies");

    assert_eq!(next.systems().len(), 2);
    assert!(
        next.system(SystemId::new(0))
            .expect("system exists")
            .ships()
            .is_empty()
    );
    assert_eq!(next.homeworld(Player::One), SystemId::new(0));
}

#[test]
fn apply_catastrophe_action_removes_one_color_from_one_system() {
    let selected = StarSystem::new(
        vec![
            Piece::new(Color::Red, Size::Small),
            Piece::new(Color::Blue, Size::Medium),
        ],
        vec![
            owned_ship(Player::One, Color::Red, Size::Small),
            owned_ship(Player::Two, Color::Red, Size::Medium),
            owned_ship(Player::One, Color::Red, Size::Large),
            owned_ship(Player::One, Color::Blue, Size::Small),
        ],
    )
    .expect("system is valid");
    let untouched = StarSystem::new(
        vec![Piece::new(Color::Red, Size::Large)],
        vec![owned_ship(Player::Two, Color::Red, Size::Small)],
    )
    .expect("system is valid");
    let state = state_with_systems_and_homeworlds(
        Bank::new(),
        vec![selected, untouched.clone()],
        [SystemId::new(0), SystemId::new(1)],
    );
    let action = Action::Catastrophe {
        system: SystemId::new(0),
        color: Color::Red,
    };

    let next = apply_action(&state, &action).expect("action applies");
    let system = next.system(SystemId::new(0)).expect("system exists");

    assert_eq!(system.stars(), &[Piece::new(Color::Blue, Size::Medium)]);
    assert_eq!(
        system.ships(),
        &[owned_ship(Player::One, Color::Blue, Size::Small)]
    );
    assert_eq!(next.system(SystemId::new(1)), Some(&untouched));
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
fn apply_catastrophe_action_prunes_a_starless_non_homeworld_with_ships() {
    let remaining_ship = owned_ship(Player::One, Color::Blue, Size::Small);
    let state = state_with_systems_and_homeworlds(
        Bank::new(),
        vec![
            StarSystem::new(
                vec![Piece::new(Color::Yellow, Size::Small)],
                vec![owned_ship(Player::One, Color::Green, Size::Small)],
            )
            .expect("system is valid"),
            StarSystem::new(
                vec![
                    Piece::new(Color::Red, Size::Small),
                    Piece::new(Color::Red, Size::Medium),
                ],
                vec![
                    owned_ship(Player::One, Color::Red, Size::Small),
                    owned_ship(Player::Two, Color::Red, Size::Large),
                    remaining_ship,
                ],
            )
            .expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Blue, Size::Large)],
                vec![owned_ship(Player::Two, Color::Blue, Size::Medium)],
            )
            .expect("system is valid"),
        ],
        [SystemId::new(0), SystemId::new(2)],
    );
    let action = Action::Catastrophe {
        system: SystemId::new(1),
        color: Color::Red,
    };

    let next = apply_action(&state, &action).expect("action applies");

    assert_eq!(next.systems().len(), 2);
    assert_eq!(next.homeworld(Player::Two), SystemId::new(1));
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
fn apply_catastrophe_action_purges_a_starless_homeworld() {
    let remaining_ship = owned_ship(Player::One, Color::Blue, Size::Small);
    let state = state_with_systems_and_homeworlds(
        Bank::new(),
        vec![
            StarSystem::new(
                vec![
                    Piece::new(Color::Red, Size::Small),
                    Piece::new(Color::Red, Size::Medium),
                ],
                vec![
                    owned_ship(Player::One, Color::Red, Size::Small),
                    owned_ship(Player::Two, Color::Red, Size::Large),
                    remaining_ship,
                ],
            )
            .expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Blue, Size::Large)],
                vec![owned_ship(Player::Two, Color::Blue, Size::Medium)],
            )
            .expect("system is valid"),
        ],
        [SystemId::new(0), SystemId::new(1)],
    );
    let action = Action::Catastrophe {
        system: SystemId::new(0),
        color: Color::Red,
    };

    let next = apply_action(&state, &action).expect("action applies");
    let homeworld = next.system(SystemId::new(0)).expect("system exists");

    assert_eq!(next.systems().len(), 2);
    assert!(homeworld.stars().is_empty());
    assert!(homeworld.ships().is_empty());
    assert_eq!(
        next.bank().count(Color::Blue, Size::Small),
        Bank::copies_per_piece() + 1
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
    state_with_primary_ships_and_bank(
        bank,
        vec![
            owned_ship(Player::One, Color::Blue, Size::Small),
            owned_ship(Player::One, Color::Green, Size::Small),
            owned_ship(Player::One, Color::Yellow, Size::Small),
            owned_ship(Player::Two, Color::Red, Size::Medium),
        ],
    )
}

fn state_with_primary_ships(ships: Vec<Piece>) -> GameState {
    state_with_primary_ships_and_bank(Bank::new(), ships)
}

fn state_with_primary_ships_and_bank(bank: Bank, ships: Vec<Piece>) -> GameState {
    state_with_systems_and_homeworlds(
        bank,
        vec![
            StarSystem::new(vec![Piece::new(Color::Yellow, Size::Small)], ships)
                .expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Green, Size::Medium)],
                vec![owned_ship(Player::Two, Color::Yellow, Size::Small)],
            )
            .expect("system is valid"),
        ],
        [SystemId::new(0), SystemId::new(1)],
    )
}

fn state_with_systems_and_homeworlds(
    bank: Bank,
    systems: Vec<StarSystem>,
    homeworlds: [SystemId; Player::COUNT],
) -> GameState {
    GameState::new(systems, homeworlds, bank).expect("state is valid")
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
