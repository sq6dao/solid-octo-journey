use super::*;
use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, StarSystemError, SystemId};

#[test]
fn actions_report_their_kind() {
    assert_eq!(
        build_action(Player::One, SystemId::new(0), Color::Green, Size::Small).kind(),
        ActionKind::Build
    );
    assert_eq!(
        Action::Move {
            player: Player::One,
            from: SystemId::new(0),
            ship: owned_ship(Player::One, Color::Blue, Size::Small),
            target: MoveTarget::Existing(SystemId::new(1)),
        }
        .kind(),
        ActionKind::Move
    );
    assert_eq!(
        Action::Trade {
            player: Player::One,
            system: SystemId::new(0),
            from: owned_ship(Player::One, Color::Blue, Size::Small),
            to: owned_ship(Player::One, Color::Green, Size::Small),
        }
        .kind(),
        ActionKind::Trade
    );
    assert_eq!(
        Action::Move {
            player: Player::One,
            from: SystemId::new(0),
            ship: owned_ship(Player::One, Color::Blue, Size::Small),
            target: MoveTarget::New {
                stars: vec![Piece::new(Color::Yellow, Size::Small)],
            },
        }
        .kind(),
        ActionKind::Move
    );
    assert_eq!(
        Action::Sacrifice {
            player: Player::One,
            system: SystemId::new(0),
            ship: owned_ship(Player::One, Color::Blue, Size::Small),
        }
        .kind(),
        ActionKind::Sacrifice
    );
    assert_eq!(
        Action::Catastrophe {
            system: SystemId::new(0),
            color: Color::Blue,
        }
        .kind(),
        ActionKind::Catastrophe
    );
}

#[test]
fn build_validation_accepts_a_well_formed_action() {
    let state = valid_state();
    let action = build_action(Player::One, SystemId::new(0), Color::Green, Size::Small);

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn build_validation_rejects_an_unknown_system() {
    let state = valid_state();
    let action = build_action(Player::One, SystemId::new(2), Color::Green, Size::Small);

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::UnknownSystem {
            system: SystemId::new(2),
        })
    );
}

#[test]
fn build_validation_rejects_an_unowned_ship() {
    let state = valid_state();
    let action = Action::Build {
        player: Player::One,
        system: SystemId::new(0),
        ship: Piece::new(Color::Green, Size::Small),
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::UnownedShip {
            ship: Piece::new(Color::Green, Size::Small),
        })
    );
}

#[test]
fn build_validation_rejects_the_wrong_owner() {
    let state = valid_state();
    let ship = owned_ship(Player::Two, Color::Green, Size::Small);
    let action = Action::Build {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::WrongOwner {
            player: Player::One,
            ship,
        })
    );
}

#[test]
fn build_validation_rejects_an_unavailable_bank_piece() {
    let state = state_with_empty_bank_piece(Color::Green, Size::Small);
    let ship = owned_ship(Player::One, Color::Green, Size::Small);
    let action = Action::Build {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::PieceUnavailable { piece: ship })
    );
}

#[test]
fn build_validation_rejects_missing_green_power() {
    let state = state_with_primary_ships(vec![
        owned_ship(Player::One, Color::Blue, Size::Small),
        owned_ship(Player::One, Color::Yellow, Size::Small),
    ]);
    let action = build_action(Player::One, SystemId::new(0), Color::Green, Size::Small);

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::MissingActionPower {
            player: Player::One,
            system: SystemId::new(0),
            color: Color::Green,
        })
    );
}

#[test]
fn build_validation_rejects_skipping_a_smaller_available_piece() {
    let state = valid_state();
    let requested = owned_ship(Player::One, Color::Green, Size::Medium);
    let smallest = owned_ship(Player::One, Color::Green, Size::Small);
    let action = Action::Build {
        player: Player::One,
        system: SystemId::new(0),
        ship: requested,
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::BuildSizeUnavailable {
            requested,
            smallest,
        })
    );
}

#[test]
fn build_validation_accepts_medium_after_small_is_exhausted() {
    let state = state_with_empty_bank_piece(Color::Green, Size::Small);
    let action = build_action(Player::One, SystemId::new(0), Color::Green, Size::Medium);

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn build_validation_accepts_large_after_smaller_pieces_are_exhausted() {
    let mut bank = Bank::new();
    exhaust_bank_piece(&mut bank, Color::Green, Size::Small);
    exhaust_bank_piece(&mut bank, Color::Green, Size::Medium);
    let state = state_with_bank(bank);
    let action = build_action(Player::One, SystemId::new(0), Color::Green, Size::Large);

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn move_validation_accepts_a_well_formed_action() {
    let state = valid_state();
    let action = move_action(
        Player::One,
        SystemId::new(0),
        SystemId::new(1),
        Color::Blue,
        Size::Small,
    );

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn move_validation_rejects_unknown_systems() {
    let state = valid_state();
    let action = move_action(
        Player::One,
        SystemId::new(0),
        SystemId::new(2),
        Color::Blue,
        Size::Small,
    );

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::UnknownSystem {
            system: SystemId::new(2),
        })
    );
}

#[test]
fn move_validation_rejects_the_same_source_and_destination() {
    let state = valid_state();
    let action = move_action(
        Player::One,
        SystemId::new(0),
        SystemId::new(0),
        Color::Blue,
        Size::Small,
    );

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::SameSystem {
            system: SystemId::new(0),
        })
    );
}

#[test]
fn move_validation_rejects_the_wrong_owner() {
    let state = valid_state();
    let ship = owned_ship(Player::Two, Color::Blue, Size::Small);
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: MoveTarget::Existing(SystemId::new(1)),
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::WrongOwner {
            player: Player::One,
            ship,
        })
    );
}

#[test]
fn move_validation_rejects_a_missing_source_ship() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Green, Size::Large);
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: MoveTarget::Existing(SystemId::new(1)),
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::ShipNotPresent {
            system: SystemId::new(0),
            ship,
        })
    );
}

#[test]
fn move_validation_rejects_missing_yellow_power() {
    let state = state_with_primary_ships(vec![
        owned_ship(Player::One, Color::Blue, Size::Small),
        owned_ship(Player::One, Color::Green, Size::Small),
    ]);
    let action = move_action(
        Player::One,
        SystemId::new(0),
        SystemId::new(1),
        Color::Blue,
        Size::Small,
    );

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::MissingActionPower {
            player: Player::One,
            system: SystemId::new(0),
            color: Color::Yellow,
        })
    );
}

#[test]
fn trade_validation_accepts_a_well_formed_action() {
    let state = valid_state();
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from: owned_ship(Player::One, Color::Blue, Size::Small),
        to: owned_ship(Player::One, Color::Green, Size::Small),
    };

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn trade_validation_rejects_a_missing_source_ship() {
    let state = valid_state();
    let from = owned_ship(Player::One, Color::Red, Size::Small);
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from,
        to: owned_ship(Player::One, Color::Yellow, Size::Small),
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::ShipNotPresent {
            system: SystemId::new(0),
            ship: from,
        })
    );
}

#[test]
fn trade_validation_rejects_the_wrong_owner() {
    let state = valid_state();
    let from = owned_ship(Player::Two, Color::Blue, Size::Small);
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from,
        to: owned_ship(Player::One, Color::Green, Size::Small),
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::WrongOwner {
            player: Player::One,
            ship: from,
        })
    );
}

#[test]
fn trade_validation_rejects_size_mismatch() {
    let state = valid_state();
    let from = owned_ship(Player::One, Color::Blue, Size::Small);
    let to = owned_ship(Player::One, Color::Green, Size::Medium);
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from,
        to,
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::SizeMismatch { from, to })
    );
}

#[test]
fn trade_validation_rejects_an_unavailable_bank_piece() {
    let state = state_with_empty_bank_piece(Color::Green, Size::Small);
    let to = owned_ship(Player::One, Color::Green, Size::Small);
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from: owned_ship(Player::One, Color::Blue, Size::Small),
        to,
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::PieceUnavailable { piece: to })
    );
}

#[test]
fn trade_validation_rejects_missing_blue_power() {
    let state = state_with_primary_ships(vec![
        owned_ship(Player::One, Color::Green, Size::Small),
        owned_ship(Player::One, Color::Yellow, Size::Small),
    ]);
    let from = owned_ship(Player::One, Color::Green, Size::Small);
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from,
        to: owned_ship(Player::One, Color::Yellow, Size::Small),
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::MissingActionPower {
            player: Player::One,
            system: SystemId::new(0),
            color: Color::Blue,
        })
    );
}

#[test]
fn move_validation_accepts_a_new_system_target() {
    let state = valid_state();
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: MoveTarget::New {
            stars: vec![Piece::new(Color::Yellow, Size::Small)],
        },
    };

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn move_validation_rejects_invalid_new_system_stars() {
    let state = valid_state();
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: MoveTarget::New {
            stars: vec![Piece::owned(Color::Yellow, Size::Small, Player::One)],
        },
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::InvalidDiscovery {
            error: StarSystemError::OwnedStar,
        })
    );
}

#[test]
fn move_validation_rejects_missing_yellow_power_for_a_new_system_target() {
    let state = state_with_primary_ships(vec![
        owned_ship(Player::One, Color::Blue, Size::Small),
        owned_ship(Player::One, Color::Green, Size::Small),
    ]);
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: MoveTarget::New {
            stars: vec![Piece::new(Color::Yellow, Size::Small)],
        },
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::MissingActionPower {
            player: Player::One,
            system: SystemId::new(0),
            color: Color::Yellow,
        })
    );
}

#[test]
fn move_validation_rejects_a_missing_source_ship_for_a_new_system_target() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Green, Size::Large);
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: MoveTarget::New {
            stars: vec![Piece::new(Color::Yellow, Size::Small)],
        },
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::ShipNotPresent {
            system: SystemId::new(0),
            ship,
        })
    );
}

#[test]
fn move_validation_rejects_the_wrong_owner_for_a_new_system_target() {
    let state = valid_state();
    let ship = owned_ship(Player::Two, Color::Blue, Size::Small);
    let action = Action::Move {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: MoveTarget::New {
            stars: vec![Piece::new(Color::Yellow, Size::Small)],
        },
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::WrongOwner {
            player: Player::One,
            ship,
        })
    );
}

#[test]
fn sacrifice_and_catastrophe_are_explicitly_unsupported() {
    let state = valid_state();
    let sacrifice = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
    };
    let catastrophe = Action::Catastrophe {
        system: SystemId::new(0),
        color: Color::Blue,
    };

    assert_eq!(
        validate_action(&state, &sacrifice),
        Err(ActionError::UnsupportedAction {
            kind: ActionKind::Sacrifice,
        })
    );
    assert_eq!(
        validate_action(&state, &catastrophe),
        Err(ActionError::UnsupportedAction {
            kind: ActionKind::Catastrophe,
        })
    );
}

fn build_action(player: Player, system: SystemId, color: Color, size: Size) -> Action {
    Action::Build {
        player,
        system,
        ship: owned_ship(player, color, size),
    }
}

fn move_action(player: Player, from: SystemId, to: SystemId, color: Color, size: Size) -> Action {
    Action::Move {
        player,
        from,
        ship: owned_ship(player, color, size),
        target: MoveTarget::Existing(to),
    }
}

fn valid_state() -> GameState {
    state_with_bank(Bank::new())
}

fn state_with_empty_bank_piece(color: Color, size: Size) -> GameState {
    let mut bank = Bank::new();
    exhaust_bank_piece(&mut bank, color, size);
    state_with_bank(bank)
}

fn exhaust_bank_piece(bank: &mut Bank, color: Color, size: Size) {
    for _ in 0..Bank::copies_per_piece() {
        bank.draw(color, size).expect("piece exists");
    }
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
    GameState::new(
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
        bank,
    )
    .expect("state is valid")
}

fn owned_ship(player: Player, color: Color, size: Size) -> Piece {
    Piece::owned(color, size, player)
}
