use hw_core::{Color, Piece, Size, SystemId};
use hw_engine::{Action, ActionKind, Game, GameStatus, TravelTarget};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AiDecision {
    Action(Action),
    EndTurn,
}

pub fn legal_decisions(game: &Game) -> Vec<AiDecision> {
    if game.status() != GameStatus::InProgress {
        return Vec::new();
    }

    let mut decisions = Vec::new();
    push_end_turn_decision(game, &mut decisions);
    push_build_decisions(game, &mut decisions);
    push_travel_decisions(game, &mut decisions);
    push_trade_decisions(game, &mut decisions);
    push_sacrifice_decisions(game, &mut decisions);
    push_invade_decisions(game, &mut decisions);
    decisions
}

fn push_end_turn_decision(game: &Game, decisions: &mut Vec<AiDecision>) {
    if game.end_turn().is_ok() {
        decisions.push(AiDecision::EndTurn);
    }
}

fn push_build_decisions(game: &Game, decisions: &mut Vec<AiDecision>) {
    if !paid_actions_allowed(game, ActionKind::Build) {
        return;
    }

    let player = game.turn().current_player();
    let state = game.turn().state();

    for system_index in 0..state.systems().len() {
        let system = SystemId::new(system_index);
        for color in Color::ALL {
            for size in Size::ALL {
                if state.bank().count(color, size) == 0 {
                    continue;
                }

                push_legal_action(
                    game,
                    decisions,
                    Action::Build {
                        player,
                        system,
                        ship: Piece::owned(color, size, player),
                    },
                );
            }
        }
    }
}

fn push_travel_decisions(game: &Game, decisions: &mut Vec<AiDecision>) {
    if !paid_actions_allowed(game, ActionKind::Travel) {
        return;
    }

    let player = game.turn().current_player();
    let state = game.turn().state();

    for (from_index, system) in state.systems().iter().enumerate() {
        let from = SystemId::new(from_index);
        for ship in system
            .ships()
            .iter()
            .copied()
            .filter(|ship| ship.is_owned_by(player))
        {
            for to_index in 0..state.systems().len() {
                if to_index == from_index {
                    continue;
                }

                push_legal_action(
                    game,
                    decisions,
                    Action::Travel {
                        player,
                        from,
                        ship,
                        target: TravelTarget::Existing(SystemId::new(to_index)),
                    },
                );
            }

            for color in Color::ALL {
                for size in Size::ALL {
                    if state.bank().count(color, size) == 0 {
                        continue;
                    }

                    push_legal_action(
                        game,
                        decisions,
                        Action::Travel {
                            player,
                            from,
                            ship,
                            target: TravelTarget::New {
                                stars: vec![Piece::new(color, size)],
                            },
                        },
                    );
                }
            }
        }
    }
}

fn push_trade_decisions(game: &Game, decisions: &mut Vec<AiDecision>) {
    if !paid_actions_allowed(game, ActionKind::Trade) {
        return;
    }

    let player = game.turn().current_player();
    let state = game.turn().state();

    for (system_index, system_ref) in state.systems().iter().enumerate() {
        let system = SystemId::new(system_index);
        for from in system_ref
            .ships()
            .iter()
            .copied()
            .filter(|ship| ship.is_owned_by(player))
        {
            for color in Color::ALL {
                if color == from.color() || state.bank().count(color, from.size()) == 0 {
                    continue;
                }

                push_legal_action(
                    game,
                    decisions,
                    Action::Trade {
                        player,
                        system,
                        from,
                        to: Piece::owned(color, from.size(), player),
                    },
                );
            }
        }
    }
}

fn push_sacrifice_decisions(game: &Game, decisions: &mut Vec<AiDecision>) {
    if !paid_actions_allowed(game, ActionKind::Sacrifice) {
        return;
    }

    let player = game.turn().current_player();
    let state = game.turn().state();

    for (system_index, system_ref) in state.systems().iter().enumerate() {
        let system = SystemId::new(system_index);
        for ship in system_ref
            .ships()
            .iter()
            .copied()
            .filter(|ship| ship.is_owned_by(player))
        {
            push_legal_action(
                game,
                decisions,
                Action::Sacrifice {
                    player,
                    system,
                    ship,
                },
            );
        }
    }
}

fn push_invade_decisions(game: &Game, decisions: &mut Vec<AiDecision>) {
    if !paid_actions_allowed(game, ActionKind::Invade) {
        return;
    }

    let player = game.turn().current_player();
    let state = game.turn().state();

    for (system_index, system_ref) in state.systems().iter().enumerate() {
        let system = SystemId::new(system_index);
        for target in system_ref
            .ships()
            .iter()
            .copied()
            .filter(|ship| ship.owner().is_some_and(|owner| owner != player))
        {
            push_legal_action(
                game,
                decisions,
                Action::Invade {
                    player,
                    system,
                    target,
                },
            );
        }
    }
}

fn paid_actions_allowed(game: &Game, action_kind: ActionKind) -> bool {
    game.turn().remaining_actions() > 0
        && game
            .turn()
            .required_action()
            .is_none_or(|required| required == action_kind)
}

fn push_legal_action(game: &Game, decisions: &mut Vec<AiDecision>, action: Action) {
    let decision = AiDecision::Action(action);
    let AiDecision::Action(action) = &decision else {
        unreachable!("constructed an action decision")
    };

    if game.apply_action(action).is_ok() && !decisions.contains(&decision) {
        decisions.push(decision);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hw_core::Player;
    use hw_engine::{GameOutcome, HomeworldSetup, TurnState};

    #[test]
    fn terminal_games_have_no_legal_decisions() {
        let game = Game::from_parts(
            Game::default(Player::One).turn().clone(),
            GameStatus::Finished(GameOutcome::Winner(Player::One)),
        );

        assert_eq!(legal_decisions(&game), Vec::new());
    }

    #[test]
    fn end_turn_is_included_only_when_engine_accepts_it() {
        let game_with_budget = Game::default(Player::One);
        assert!(!legal_decisions(&game_with_budget).contains(&AiDecision::EndTurn));

        let spent_turn = TurnState::from_parts(
            game_with_budget.turn().state().clone(),
            Player::One,
            0,
            None,
        );
        let game_without_budget = Game::from_parts(spent_turn, GameStatus::InProgress);

        assert_eq!(
            legal_decisions(&game_without_budget),
            vec![AiDecision::EndTurn]
        );
    }

    #[test]
    fn build_decisions_are_legal_and_deterministic() {
        let game = Game::default(Player::One);
        let decisions = legal_decisions(&game);

        assert_all_actions_apply(&game, &decisions);
        let build_kinds = decisions
            .iter()
            .filter_map(|decision| match decision {
                AiDecision::Action(Action::Build { .. }) => Some(ActionKind::Build),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            build_kinds,
            vec![
                ActionKind::Build,
                ActionKind::Build,
                ActionKind::Build,
                ActionKind::Build
            ]
        );
        assert!(decisions.contains(&AiDecision::Action(Action::Build {
            player: Player::One,
            system: SystemId::new(0),
            ship: Piece::owned(Color::Red, Size::Small, Player::One),
        })));
    }

    #[test]
    fn build_decisions_respect_required_action_kind() {
        let game = Game::default(Player::One);
        let turn = TurnState::from_parts(
            game.turn().state().clone(),
            Player::One,
            1,
            Some(ActionKind::Travel),
        );
        let game = Game::from_parts(turn, GameStatus::InProgress);

        assert!(
            legal_decisions(&game)
                .iter()
                .all(|decision| !matches!(decision, AiDecision::Action(Action::Build { .. })))
        );
    }

    #[test]
    fn travel_decisions_include_existing_system_targets() {
        let game = existing_travel_game();
        let decisions = legal_decisions(&game);

        assert_all_actions_apply(&game, &decisions);
        assert!(decisions.contains(&AiDecision::Action(Action::Travel {
            player: Player::One,
            from: SystemId::new(0),
            ship: Piece::owned(Color::Green, Size::Small, Player::One),
            target: TravelTarget::Existing(SystemId::new(1)),
        })));
    }

    #[test]
    fn travel_decisions_discover_only_one_star_systems() {
        let game = Game::default(Player::One);
        let decisions = legal_decisions(&game);
        let mut found_discovery = false;

        for decision in decisions {
            if let AiDecision::Action(Action::Travel {
                target: TravelTarget::New { stars },
                ..
            }) = decision
            {
                found_discovery = true;
                assert_eq!(stars.len(), 1);
            }
        }

        assert!(found_discovery);
    }

    #[test]
    fn trade_decisions_include_same_size_other_color_bank_ships() {
        let game = Game::default(Player::One);
        let decisions = legal_decisions(&game);

        assert_all_actions_apply(&game, &decisions);
        assert!(decisions.contains(&AiDecision::Action(Action::Trade {
            player: Player::One,
            system: SystemId::new(0),
            from: Piece::owned(Color::Green, Size::Small, Player::One),
            to: Piece::owned(Color::Red, Size::Small, Player::One),
        })));

        for decision in decisions {
            if let AiDecision::Action(Action::Trade { from, to, .. }) = decision {
                assert_eq!(from.size(), to.size());
                assert_ne!(from.color(), to.color());
            }
        }
    }

    #[test]
    fn sacrifice_decisions_include_owned_ships() {
        let game = Game::default(Player::One);
        let decisions = legal_decisions(&game);

        assert_all_actions_apply(&game, &decisions);
        assert!(decisions.contains(&AiDecision::Action(Action::Sacrifice {
            player: Player::One,
            system: SystemId::new(0),
            ship: Piece::owned(Color::Green, Size::Small, Player::One),
        })));
    }

    #[test]
    fn paid_actions_are_absent_without_budget() {
        let game = Game::default(Player::One);
        let turn = TurnState::from_parts(game.turn().state().clone(), Player::One, 0, None);
        let game = Game::from_parts(turn, GameStatus::InProgress);

        assert_eq!(legal_decisions(&game), vec![AiDecision::EndTurn]);
    }

    #[test]
    fn paid_actions_respect_sacrifice_action_kind_restrictions() {
        let game = Game::default(Player::One);
        let turn = TurnState::from_parts(
            game.turn().state().clone(),
            Player::One,
            2,
            Some(ActionKind::Build),
        );
        let game = Game::from_parts(turn, GameStatus::InProgress);

        let decisions = legal_decisions(&game);
        assert!(!decisions.is_empty());
        assert!(decisions.iter().all(|decision| match decision {
            AiDecision::Action(action) => action.kind() == ActionKind::Build,
            AiDecision::EndTurn => false,
        }));
    }

    #[test]
    fn invade_decisions_include_opponent_ships() {
        let game = invade_game();
        let decisions = legal_decisions(&game);

        assert_all_actions_apply(&game, &decisions);
        assert!(decisions.contains(&AiDecision::Action(Action::Invade {
            player: Player::One,
            system: SystemId::new(0),
            target: Piece::owned(Color::Green, Size::Small, Player::Two),
        })));
    }

    fn assert_all_actions_apply(game: &Game, decisions: &[AiDecision]) {
        for decision in decisions {
            if let AiDecision::Action(action) = decision {
                game.apply_action(action)
                    .expect("generated action should apply");
            }
        }
    }

    fn existing_travel_game() -> Game {
        Game::new(
            [
                HomeworldSetup::new(
                    vec![
                        Piece::new(Color::Yellow, Size::Small),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    Piece::owned(Color::Green, Size::Small, Player::One),
                ),
                HomeworldSetup::new(
                    vec![
                        Piece::new(Color::Red, Size::Large),
                        Piece::new(Color::Green, Size::Large),
                    ],
                    Piece::owned(Color::Red, Size::Small, Player::Two),
                ),
            ],
            Player::One,
        )
        .expect("game initializes")
    }

    fn invade_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Red, Size::Small)],
                    vec![
                        Piece::owned(Color::Red, Size::Large, Player::One),
                        Piece::owned(Color::Green, Size::Small, Player::Two),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![
                        Piece::new(Color::Blue, Size::Medium),
                        Piece::new(Color::Yellow, Size::Large),
                    ],
                    vec![Piece::owned(Color::Blue, Size::Small, Player::Two)],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");

        Game::from_parts(TurnState::new(state, Player::One), GameStatus::InProgress)
    }
}
