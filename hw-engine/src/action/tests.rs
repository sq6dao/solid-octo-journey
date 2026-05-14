use super::*;
use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, StarSystemError, SystemId};

#[test]
fn actions_report_their_kind() {
    assert_eq!(
        build_action(Player::One, SystemId::new(0), Color::Green, Size::Small).kind(),
        ActionKind::Build
    );
    assert_eq!(
        Action::Travel {
            player: Player::One,
            from: SystemId::new(0),
            ship: owned_ship(Player::One, Color::Blue, Size::Small),
            target: TravelTarget::Existing(SystemId::new(1)),
        }
        .kind(),
        ActionKind::Travel
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
        Action::Travel {
            player: Player::One,
            from: SystemId::new(0),
            ship: owned_ship(Player::One, Color::Blue, Size::Small),
            target: TravelTarget::New {
                stars: vec![Piece::new(Color::Yellow, Size::Small)],
            },
        }
        .kind(),
        ActionKind::Travel
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
        Action::Invade {
            player: Player::One,
            system: SystemId::new(0),
            target: owned_ship(Player::Two, Color::Blue, Size::Small),
        }
        .kind(),
        ActionKind::Invade
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
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Yellow, Size::Small)],
        vec![
            owned_ship(Player::One, Color::Blue, Size::Small),
            owned_ship(Player::One, Color::Yellow, Size::Small),
        ],
    );
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
fn build_validation_accepts_green_star_power() {
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Green, Size::Small)],
        vec![owned_ship(Player::One, Color::Blue, Size::Small)],
    );
    let action = build_action(Player::One, SystemId::new(0), Color::Yellow, Size::Small);

    assert_eq!(validate_action(&state, &action), Ok(()));
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
fn travel_validation_accepts_a_well_formed_action() {
    let state = valid_state();
    let action = travel_action(
        Player::One,
        SystemId::new(0),
        SystemId::new(1),
        Color::Blue,
        Size::Small,
    );

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn travel_validation_rejects_unknown_systems() {
    let state = valid_state();
    let action = travel_action(
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
fn travel_validation_rejects_the_same_source_and_destination() {
    let state = valid_state();
    let action = travel_action(
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
fn travel_validation_rejects_the_wrong_owner() {
    let state = valid_state();
    let ship = owned_ship(Player::Two, Color::Blue, Size::Small);
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: TravelTarget::Existing(SystemId::new(1)),
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
fn travel_validation_rejects_a_missing_source_ship() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Green, Size::Large);
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: TravelTarget::Existing(SystemId::new(1)),
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
fn travel_validation_rejects_missing_yellow_power() {
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Green, Size::Small)],
        vec![
            owned_ship(Player::One, Color::Blue, Size::Small),
            owned_ship(Player::One, Color::Green, Size::Small),
        ],
    );
    let action = travel_action(
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
fn travel_validation_accepts_yellow_star_power() {
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Yellow, Size::Small)],
        vec![owned_ship(Player::One, Color::Blue, Size::Small)],
    );
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: TravelTarget::New {
            stars: vec![Piece::new(Color::Red, Size::Medium)],
        },
    };

    assert_eq!(validate_action(&state, &action), Ok(()));
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
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Yellow, Size::Small)],
        vec![
            owned_ship(Player::One, Color::Green, Size::Small),
            owned_ship(Player::One, Color::Yellow, Size::Small),
        ],
    );
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
fn trade_validation_accepts_blue_star_power() {
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Blue, Size::Small)],
        vec![owned_ship(Player::One, Color::Green, Size::Small)],
    );
    let action = Action::Trade {
        player: Player::One,
        system: SystemId::new(0),
        from: owned_ship(Player::One, Color::Green, Size::Small),
        to: owned_ship(Player::One, Color::Yellow, Size::Small),
    };

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn travel_validation_accepts_a_new_system_target() {
    let state = valid_state();
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: TravelTarget::New {
            stars: vec![Piece::new(Color::Red, Size::Medium)],
        },
    };

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn travel_validation_rejects_existing_target_with_a_shared_star_size() {
    let state = state_with_bank_and_systems(
        Bank::new(),
        vec![
            StarSystem::new(
                vec![Piece::new(Color::Yellow, Size::Small)],
                vec![
                    owned_ship(Player::One, Color::Blue, Size::Small),
                    owned_ship(Player::One, Color::Yellow, Size::Small),
                ],
            )
            .expect("system is valid"),
            StarSystem::new(
                vec![Piece::new(Color::Red, Size::Small)],
                vec![owned_ship(Player::Two, Color::Green, Size::Small)],
            )
            .expect("system is valid"),
        ],
    );
    let action = travel_action(
        Player::One,
        SystemId::new(0),
        SystemId::new(1),
        Color::Blue,
        Size::Small,
    );

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::StarSizeConflict { size: Size::Small })
    );
}

#[test]
fn travel_validation_rejects_new_target_with_a_shared_star_size() {
    let state = valid_state();
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: TravelTarget::New {
            stars: vec![Piece::new(Color::Red, Size::Small)],
        },
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::StarSizeConflict { size: Size::Small })
    );
}

#[test]
fn travel_validation_rejects_invalid_new_system_stars() {
    let state = valid_state();
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: TravelTarget::New {
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
fn travel_validation_rejects_an_unavailable_discovery_star() {
    let state = state_with_empty_bank_piece(Color::Red, Size::Large);
    let star = Piece::new(Color::Red, Size::Large);
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: TravelTarget::New { stars: vec![star] },
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::PieceUnavailable { piece: star })
    );
}

#[test]
fn travel_validation_rejects_duplicate_discovery_stars_exceeding_the_bank() {
    let mut bank = Bank::new();
    bank.draw(Color::Red, Size::Large).expect("piece exists");
    bank.draw(Color::Red, Size::Large).expect("piece exists");
    let state = state_with_bank(bank);
    let star = Piece::new(Color::Red, Size::Large);
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: TravelTarget::New {
            stars: vec![star, star],
        },
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::PieceUnavailable { piece: star })
    );
}

#[test]
fn travel_validation_rejects_missing_yellow_power_for_a_new_system_target() {
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Green, Size::Small)],
        vec![
            owned_ship(Player::One, Color::Blue, Size::Small),
            owned_ship(Player::One, Color::Green, Size::Small),
        ],
    );
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
        target: TravelTarget::New {
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
fn travel_validation_rejects_a_missing_source_ship_for_a_new_system_target() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Green, Size::Large);
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: TravelTarget::New {
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
fn travel_validation_rejects_the_wrong_owner_for_a_new_system_target() {
    let state = valid_state();
    let ship = owned_ship(Player::Two, Color::Blue, Size::Small);
    let action = Action::Travel {
        player: Player::One,
        from: SystemId::new(0),
        ship,
        target: TravelTarget::New {
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
fn catastrophe_validation_accepts_four_same_color_pieces() {
    let state = state_with_catastrophe_count(Color::Red, 4);
    let action = Action::Catastrophe {
        system: SystemId::new(0),
        color: Color::Red,
    };

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn catastrophe_validation_counts_stars_and_ships() {
    let state = state_with_bank_and_systems(
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
                ],
            )
            .expect("system is valid"),
            secondary_system(),
        ],
    );
    let action = Action::Catastrophe {
        system: SystemId::new(0),
        color: Color::Red,
    };

    assert_eq!(validate_action(&state, &action), Ok(()));
}

#[test]
fn catastrophe_validation_rejects_zero_to_three_same_color_pieces() {
    for count in 0..4 {
        let state = state_with_catastrophe_count(Color::Red, count);
        let action = Action::Catastrophe {
            system: SystemId::new(0),
            color: Color::Red,
        };

        assert_eq!(
            validate_action(&state, &action),
            Err(ActionError::NoCatastrophe {
                system: SystemId::new(0),
                color: Color::Red,
                count,
            })
        );
    }
}

#[test]
fn catastrophe_validation_rejects_an_unknown_system() {
    let state = valid_state();
    let action = Action::Catastrophe {
        system: SystemId::new(2),
        color: Color::Blue,
    };

    assert_eq!(
        validate_action(&state, &action),
        Err(ActionError::UnknownSystem {
            system: SystemId::new(2),
        })
    );
}

#[test]
fn possible_catastrophe_detection_accepts_four_same_color_pieces() {
    let state = state_with_catastrophe_count(Color::Red, 4);

    assert!(has_possible_catastrophe(&state));
}

#[test]
fn possible_catastrophe_detection_rejects_three_same_color_pieces() {
    let state = state_with_catastrophe_count(Color::Red, 3);

    assert!(!has_possible_catastrophe(&state));
}

#[test]
fn sacrifice_validation_accepts_an_owned_ship_present_at_the_system() {
    let state = valid_state();
    let sacrifice = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(0),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
    };

    assert_eq!(validate_action(&state, &sacrifice), Ok(()));
}

#[test]
fn sacrifice_validation_rejects_an_unknown_system() {
    let state = valid_state();
    let sacrifice = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(2),
        ship: owned_ship(Player::One, Color::Blue, Size::Small),
    };

    assert_eq!(
        validate_action(&state, &sacrifice),
        Err(ActionError::UnknownSystem {
            system: SystemId::new(2),
        })
    );
}

#[test]
fn sacrifice_validation_rejects_an_unowned_ship() {
    let state = valid_state();
    let ship = Piece::new(Color::Blue, Size::Small);
    let sacrifice = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    assert_eq!(
        validate_action(&state, &sacrifice),
        Err(ActionError::UnownedShip { ship })
    );
}

#[test]
fn sacrifice_validation_rejects_the_wrong_owner() {
    let state = valid_state();
    let ship = owned_ship(Player::Two, Color::Blue, Size::Small);
    let sacrifice = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    assert_eq!(
        validate_action(&state, &sacrifice),
        Err(ActionError::WrongOwner {
            player: Player::One,
            ship,
        })
    );
}

#[test]
fn sacrifice_validation_rejects_a_missing_ship() {
    let state = valid_state();
    let ship = owned_ship(Player::One, Color::Green, Size::Large);
    let sacrifice = Action::Sacrifice {
        player: Player::One,
        system: SystemId::new(0),
        ship,
    };

    assert_eq!(
        validate_action(&state, &sacrifice),
        Err(ActionError::ShipNotPresent {
            system: SystemId::new(0),
            ship,
        })
    );
}

#[test]
fn invade_validation_accepts_an_opponent_ship_with_red_power() {
    let state = valid_state();
    let target = owned_ship(Player::One, Color::Blue, Size::Small);
    let invade = Action::Invade {
        player: Player::Two,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(validate_action(&state, &invade), Ok(()));
}

#[test]
fn invade_validation_accepts_an_opponent_ship_of_the_same_size() {
    let target = owned_ship(Player::Two, Color::Blue, Size::Medium);
    let state = state_with_primary_ships(vec![
        owned_ship(Player::One, Color::Red, Size::Medium),
        target,
    ]);
    let invade = Action::Invade {
        player: Player::One,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(validate_action(&state, &invade), Ok(()));
}

#[test]
fn invade_validation_rejects_an_unknown_system() {
    let state = valid_state();
    let target = owned_ship(Player::One, Color::Blue, Size::Small);
    let invade = Action::Invade {
        player: Player::Two,
        system: SystemId::new(2),
        target,
    };

    assert_eq!(
        validate_action(&state, &invade),
        Err(ActionError::UnknownSystem {
            system: SystemId::new(2),
        })
    );
}

#[test]
fn invade_validation_rejects_an_unowned_target() {
    let state = valid_state();
    let target = Piece::new(Color::Blue, Size::Small);
    let invade = Action::Invade {
        player: Player::Two,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(
        validate_action(&state, &invade),
        Err(ActionError::UnownedShip { ship: target })
    );
}

#[test]
fn invade_validation_rejects_a_missing_target_ship() {
    let state = valid_state();
    let target = owned_ship(Player::One, Color::Green, Size::Large);
    let invade = Action::Invade {
        player: Player::Two,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(
        validate_action(&state, &invade),
        Err(ActionError::ShipNotPresent {
            system: SystemId::new(0),
            ship: target,
        })
    );
}

#[test]
fn invade_validation_rejects_a_target_owned_by_the_acting_player() {
    let state = valid_state();
    let target = owned_ship(Player::One, Color::Blue, Size::Small);
    let invade = Action::Invade {
        player: Player::One,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(
        validate_action(&state, &invade),
        Err(ActionError::CannotInvadeOwnShip {
            player: Player::One,
            ship: target,
        })
    );
}

#[test]
fn invade_validation_rejects_missing_red_power() {
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Yellow, Size::Small)],
        vec![
            owned_ship(Player::One, Color::Blue, Size::Small),
            owned_ship(Player::One, Color::Green, Size::Small),
            owned_ship(Player::One, Color::Yellow, Size::Small),
            owned_ship(Player::Two, Color::Red, Size::Medium),
        ],
    );
    let target = owned_ship(Player::Two, Color::Red, Size::Medium);
    let invade = Action::Invade {
        player: Player::One,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(
        validate_action(&state, &invade),
        Err(ActionError::MissingActionPower {
            player: Player::One,
            system: SystemId::new(0),
            color: Color::Red,
        })
    );
}

#[test]
fn invade_validation_accepts_red_star_power() {
    let target = owned_ship(Player::Two, Color::Red, Size::Medium);
    let state = state_with_primary_stars_and_ships(
        vec![Piece::new(Color::Red, Size::Small)],
        vec![owned_ship(Player::One, Color::Blue, Size::Medium), target],
    );
    let invade = Action::Invade {
        player: Player::One,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(validate_action(&state, &invade), Ok(()));
}

#[test]
fn invade_validation_rejects_a_larger_target_ship() {
    let target = owned_ship(Player::Two, Color::Blue, Size::Medium);
    let state = state_with_primary_ships(vec![
        owned_ship(Player::One, Color::Red, Size::Small),
        target,
    ]);
    let invade = Action::Invade {
        player: Player::One,
        system: SystemId::new(0),
        target,
    };

    assert_eq!(
        validate_action(&state, &invade),
        Err(ActionError::CannotInvadeLargerShip {
            player: Player::One,
            target,
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

fn travel_action(player: Player, from: SystemId, to: SystemId, color: Color, size: Size) -> Action {
    Action::Travel {
        player,
        from,
        ship: owned_ship(player, color, size),
        target: TravelTarget::Existing(to),
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
    state_with_primary_stars_and_ships(vec![Piece::new(Color::Yellow, Size::Small)], ships)
}

fn state_with_primary_ships_and_bank(bank: Bank, ships: Vec<Piece>) -> GameState {
    state_with_primary_stars_ships_and_bank(
        bank,
        vec![Piece::new(Color::Yellow, Size::Small)],
        ships,
    )
}

fn state_with_primary_stars_and_ships(stars: Vec<Piece>, ships: Vec<Piece>) -> GameState {
    state_with_primary_stars_ships_and_bank(Bank::new(), stars, ships)
}

fn state_with_primary_stars_ships_and_bank(
    bank: Bank,
    stars: Vec<Piece>,
    ships: Vec<Piece>,
) -> GameState {
    state_with_bank_and_systems(
        bank,
        vec![
            StarSystem::new(stars, ships).expect("system is valid"),
            secondary_system(),
        ],
    )
}

fn state_with_catastrophe_count(color: Color, count: usize) -> GameState {
    state_with_bank_and_systems(
        Bank::new(),
        vec![system_with_color_count(color, count), secondary_system()],
    )
}

fn state_with_bank_and_systems(bank: Bank, systems: Vec<StarSystem>) -> GameState {
    GameState::new(systems, [SystemId::new(0), SystemId::new(1)], bank).expect("state is valid")
}

fn system_with_color_count(color: Color, count: usize) -> StarSystem {
    let mut stars = Vec::new();
    let star_count = count.min(2);
    for size in [Size::Small, Size::Medium].into_iter().take(star_count) {
        stars.push(Piece::new(color, size));
    }

    let mut ships = Vec::new();
    for size in [Size::Small, Size::Medium, Size::Large]
        .into_iter()
        .take(count.saturating_sub(star_count))
    {
        ships.push(owned_ship(Player::One, color, size));
    }

    if ships.is_empty() {
        ships.push(owned_ship(Player::One, Color::Blue, Size::Small));
    }

    StarSystem::new(stars, ships).expect("system is valid")
}

fn secondary_system() -> StarSystem {
    StarSystem::new(
        vec![Piece::new(Color::Green, Size::Medium)],
        vec![owned_ship(Player::Two, Color::Yellow, Size::Small)],
    )
    .expect("system is valid")
}

fn owned_ship(player: Player, color: Color, size: Size) -> Piece {
    Piece::owned(color, size, player)
}
