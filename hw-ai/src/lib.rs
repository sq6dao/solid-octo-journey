mod evaluation;
mod search;

pub use evaluation::{Evaluation, evaluate_position};
pub use search::{ScoredDecision, SearchStrategy, best_decision, best_decision_at_depth};

use hw_core::{Color, Piece, Size, SystemId};
use hw_engine::{Action, ActionKind, Game, GameOutcome, GameStatus, TravelTarget};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AiDecision {
    Action(Action),
    EndTurn,
}

pub trait Strategy {
    fn choose(&self, game: &Game) -> Option<AiDecision>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FirstLegalStrategy;

impl Strategy for FirstLegalStrategy {
    fn choose(&self, game: &Game) -> Option<AiDecision> {
        legal_decisions(game)
            .into_iter()
            .find(|decision| !is_immediate_non_win(game, decision))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PriorityStrategy;

impl Strategy for PriorityStrategy {
    fn choose(&self, game: &Game) -> Option<AiDecision> {
        let decisions = legal_decisions(game)
            .into_iter()
            .filter(|decision| !is_immediate_non_win(game, decision))
            .collect::<Vec<_>>();

        first_matching(&decisions, |decision| is_immediate_win(game, decision))
            .or_else(|| first_matching(&decisions, is_catastrophe_decision))
            .or_else(|| first_matching(&decisions, is_paid_action_decision))
            .or_else(|| first_matching(&decisions, is_end_turn_decision))
    }
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
    push_catastrophe_decisions(game, &mut decisions);
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

fn push_catastrophe_decisions(game: &Game, decisions: &mut Vec<AiDecision>) {
    let state = game.turn().state();

    for system_index in 0..state.systems().len() {
        let system = SystemId::new(system_index);
        for color in Color::ALL {
            push_legal_action(game, decisions, Action::Catastrophe { system, color });
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
    if game.apply_action(&action).is_err() {
        return;
    }

    let decision = AiDecision::Action(action);
    if !decisions.contains(&decision) {
        decisions.push(decision);
    }
}

fn first_matching(
    decisions: &[AiDecision],
    mut predicate: impl FnMut(&AiDecision) -> bool,
) -> Option<AiDecision> {
    decisions
        .iter()
        .find(|decision| predicate(decision))
        .cloned()
}

fn is_immediate_win(game: &Game, decision: &AiDecision) -> bool {
    let current_player = game.turn().current_player();

    match decision {
        AiDecision::Action(action) => game.apply_action(action).is_ok_and(|next| {
            is_winning_status(next.status(), current_player)
                || next
                    .end_turn()
                    .is_ok_and(|ended| is_winning_status(ended.status(), current_player))
        }),
        AiDecision::EndTurn => game
            .end_turn()
            .is_ok_and(|next| is_winning_status(next.status(), current_player)),
    }
}

pub(crate) fn is_immediate_non_win(game: &Game, decision: &AiDecision) -> bool {
    let current_player = game.turn().current_player();

    match decision {
        AiDecision::Action(action) => game.apply_action(action).is_ok_and(|next| {
            is_terminal_non_win(next.status(), current_player)
                || next
                    .end_turn()
                    .is_ok_and(|ended| is_terminal_non_win(ended.status(), current_player))
        }),
        AiDecision::EndTurn => game
            .end_turn()
            .is_ok_and(|next| is_terminal_non_win(next.status(), current_player)),
    }
}

fn is_winning_status(status: GameStatus, player: hw_core::Player) -> bool {
    matches!(status, GameStatus::Finished(GameOutcome::Winner(winner)) if winner == player)
}

fn is_terminal_non_win(status: GameStatus, player: hw_core::Player) -> bool {
    match status {
        GameStatus::InProgress => false,
        GameStatus::Finished(GameOutcome::Winner(winner)) => winner != player,
        GameStatus::Finished(GameOutcome::Draw) => true,
    }
}

fn is_catastrophe_decision(decision: &AiDecision) -> bool {
    matches!(decision, AiDecision::Action(Action::Catastrophe { .. }))
}

fn is_paid_action_decision(decision: &AiDecision) -> bool {
    matches!(decision, AiDecision::Action(action) if action.kind() != ActionKind::Catastrophe)
}

fn is_end_turn_decision(decision: &AiDecision) -> bool {
    matches!(decision, AiDecision::EndTurn)
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
    fn first_legal_strategy_returns_first_generated_decision() {
        let game = Game::default(Player::One);

        assert_eq!(
            FirstLegalStrategy.choose(&game),
            legal_decisions(&game).into_iter().next()
        );
    }

    #[test]
    fn first_legal_strategy_returns_none_for_terminal_games() {
        let game = Game::from_parts(
            Game::default(Player::One).turn().clone(),
            GameStatus::Finished(GameOutcome::Winner(Player::One)),
        );

        assert_eq!(FirstLegalStrategy.choose(&game), None);
    }

    #[test]
    fn first_legal_strategy_skips_immediate_loss() {
        let game = last_homeworld_ship_travel_game();

        assert_eq!(
            legal_decisions(&game).into_iter().next(),
            Some(AiDecision::Action(Action::Travel {
                player: Player::One,
                from: SystemId::new(0),
                ship: Piece::owned(Color::Yellow, Size::Small, Player::One),
                target: TravelTarget::Existing(SystemId::new(1)),
            }))
        );
        assert_eq!(
            FirstLegalStrategy.choose(&game),
            Some(AiDecision::Action(Action::Catastrophe {
                system: SystemId::new(1),
                color: Color::Red,
            }))
        );
    }

    #[test]
    fn strategies_return_none_when_all_decisions_immediately_lose() {
        let game = empty_current_homeworld_game();

        assert_eq!(legal_decisions(&game), vec![AiDecision::EndTurn]);
        assert_eq!(FirstLegalStrategy.choose(&game), None);
        assert_eq!(PriorityStrategy.choose(&game), None);
    }

    #[test]
    fn strategies_return_none_when_all_decisions_immediately_draw() {
        let game = empty_homeworlds_game();

        assert_eq!(legal_decisions(&game), vec![AiDecision::EndTurn]);
        assert_eq!(FirstLegalStrategy.choose(&game), None);
        assert_eq!(PriorityStrategy.choose(&game), None);
    }

    #[test]
    fn priority_strategy_prefers_immediate_wins() {
        let game = winning_catastrophe_game();

        assert_eq!(
            PriorityStrategy.choose(&game),
            Some(AiDecision::Action(Action::Catastrophe {
                system: SystemId::new(1),
                color: Color::Red,
            }))
        );
    }

    #[test]
    fn priority_strategy_prefers_catastrophes_before_paid_actions() {
        let game = catastrophe_and_paid_action_game();

        assert_eq!(
            PriorityStrategy.choose(&game),
            Some(AiDecision::Action(Action::Catastrophe {
                system: SystemId::new(2),
                color: Color::Red,
            }))
        );
    }

    #[test]
    fn priority_strategy_skips_immediate_loss() {
        let game = losing_catastrophe_or_safe_end_turn_game();

        assert_eq!(PriorityStrategy.choose(&game), Some(AiDecision::EndTurn));
    }

    #[test]
    fn priority_strategy_prefers_paid_actions_before_absent_turn_end() {
        let game = Game::default(Player::One);

        assert_eq!(
            PriorityStrategy.choose(&game),
            Some(AiDecision::Action(Action::Build {
                player: Player::One,
                system: SystemId::new(0),
                ship: Piece::owned(Color::Red, Size::Small, Player::One),
            }))
        );
    }

    #[test]
    fn priority_strategy_uses_legal_turn_end_as_fallback() {
        let game = Game::default(Player::One);
        let turn = TurnState::from_parts(game.turn().state().clone(), Player::One, 0, None);
        let game = Game::from_parts(turn, GameStatus::InProgress);

        assert_eq!(PriorityStrategy.choose(&game), Some(AiDecision::EndTurn));
    }

    #[test]
    fn priority_strategy_returns_none_for_terminal_games() {
        let game = Game::from_parts(
            Game::default(Player::One).turn().clone(),
            GameStatus::Finished(GameOutcome::Winner(Player::One)),
        );

        assert_eq!(PriorityStrategy.choose(&game), None);
    }

    #[test]
    fn priority_strategy_uses_deterministic_tie_breaking() {
        let game = two_nonwinning_catastrophes_game();

        assert_eq!(
            PriorityStrategy.choose(&game),
            Some(AiDecision::Action(Action::Catastrophe {
                system: SystemId::new(0),
                color: Color::Red,
            }))
        );
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
        assert!(decisions.contains(&AiDecision::EndTurn));
        assert!(decisions.iter().all(|decision| match decision {
            AiDecision::Action(action) => action.kind() == ActionKind::Build,
            AiDecision::EndTurn => true,
        }));
    }

    #[test]
    fn catastrophe_decisions_are_generated_at_zero_budget() {
        let game = catastrophe_game(0);

        assert_eq!(
            legal_decisions(&game),
            vec![
                AiDecision::EndTurn,
                AiDecision::Action(Action::Catastrophe {
                    system: SystemId::new(0),
                    color: Color::Red,
                }),
            ]
        );
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

    #[test]
    fn every_generated_action_applies_for_representative_positions() {
        for game in [
            Game::default(Player::One),
            existing_travel_game(),
            invade_game(),
            catastrophe_game(1),
        ] {
            assert_all_actions_apply(&game, &legal_decisions(&game));
        }
    }

    #[test]
    fn duplicate_equivalent_decisions_are_removed() {
        let game = duplicate_ship_game();
        let sacrifice = AiDecision::Action(Action::Sacrifice {
            player: Player::One,
            system: SystemId::new(0),
            ship: Piece::owned(Color::Green, Size::Small, Player::One),
        });

        assert_eq!(
            legal_decisions(&game)
                .into_iter()
                .filter(|decision| *decision == sacrifice)
                .count(),
            1
        );
    }

    #[test]
    fn action_family_order_is_deterministic() {
        let decisions = legal_decisions(&Game::default(Player::One));
        let mut families = Vec::new();

        for family in decisions.iter().filter_map(|decision| match decision {
            AiDecision::Action(action) => Some(action.kind()),
            AiDecision::EndTurn => None,
        }) {
            if families.last() != Some(&family) {
                families.push(family);
            }
        }

        assert_eq!(
            families,
            vec![
                ActionKind::Build,
                ActionKind::Travel,
                ActionKind::Trade,
                ActionKind::Sacrifice,
            ]
        );
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

    fn last_homeworld_ship_travel_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Small)],
                    vec![Piece::owned(Color::Yellow, Size::Small, Player::One)],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Large),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    vec![
                        Piece::owned(Color::Red, Size::Small, Player::Two),
                        Piece::owned(Color::Red, Size::Medium, Player::Two),
                        Piece::owned(Color::Red, Size::Large, Player::Two),
                    ],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");

        Game::from_parts(TurnState::new(state, Player::One), GameStatus::InProgress)
    }

    fn empty_current_homeworld_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(vec![Piece::new(Color::Yellow, Size::Small)], vec![])
                    .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Blue, Size::Medium)],
                    vec![Piece::owned(Color::Blue, Size::Small, Player::Two)],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");
        let turn = TurnState::from_parts(state, Player::One, 0, None);

        Game::from_parts(turn, GameStatus::InProgress)
    }

    fn empty_homeworlds_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(vec![Piece::new(Color::Yellow, Size::Small)], vec![])
                    .expect("system is valid"),
                StarSystem::new(vec![Piece::new(Color::Blue, Size::Medium)], vec![])
                    .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");
        let turn = TurnState::from_parts(state, Player::One, 0, None);

        Game::from_parts(turn, GameStatus::InProgress)
    }

    fn losing_catastrophe_or_safe_end_turn_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Yellow, Size::Medium),
                    ],
                    vec![
                        Piece::owned(Color::Red, Size::Small, Player::One),
                        Piece::owned(Color::Red, Size::Medium, Player::One),
                        Piece::owned(Color::Red, Size::Large, Player::One),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Blue, Size::Large)],
                    vec![Piece::owned(Color::Blue, Size::Small, Player::Two)],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");
        let turn = TurnState::from_parts(state, Player::One, 0, None);

        Game::from_parts(turn, GameStatus::InProgress)
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

    fn catastrophe_game(remaining_actions: usize) -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Red, Size::Medium),
                    ],
                    vec![
                        Piece::owned(Color::Red, Size::Large, Player::One),
                        Piece::owned(Color::Red, Size::Small, Player::Two),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![
                        Piece::new(Color::Blue, Size::Small),
                        Piece::new(Color::Yellow, Size::Medium),
                    ],
                    vec![Piece::owned(Color::Blue, Size::Small, Player::Two)],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");
        let turn = TurnState::from_parts(state, Player::One, remaining_actions, None);

        Game::from_parts(turn, GameStatus::InProgress)
    }

    fn winning_catastrophe_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    vec![
                        Piece::owned(Color::Green, Size::Small, Player::One),
                        Piece::owned(Color::Red, Size::Medium, Player::One),
                        Piece::owned(Color::Red, Size::Large, Player::One),
                        Piece::owned(Color::Red, Size::Small, Player::Two),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Yellow, Size::Large),
                    ],
                    vec![
                        Piece::owned(Color::Red, Size::Small, Player::Two),
                        Piece::owned(Color::Red, Size::Medium, Player::Two),
                        Piece::owned(Color::Red, Size::Large, Player::Two),
                    ],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");
        let turn = TurnState::from_parts(state, Player::One, 0, None);

        Game::from_parts(turn, GameStatus::InProgress)
    }

    fn catastrophe_and_paid_action_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Green, Size::Small),
                        Piece::new(Color::Yellow, Size::Medium),
                    ],
                    vec![Piece::owned(Color::Green, Size::Small, Player::One)],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Blue, Size::Large),
                    ],
                    vec![Piece::owned(Color::Red, Size::Medium, Player::Two)],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Red, Size::Small)],
                    vec![
                        Piece::owned(Color::Red, Size::Small, Player::One),
                        Piece::owned(Color::Red, Size::Medium, Player::Two),
                        Piece::owned(Color::Red, Size::Large, Player::Two),
                    ],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");

        Game::from_parts(TurnState::new(state, Player::One), GameStatus::InProgress)
    }

    fn two_nonwinning_catastrophes_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    vec![
                        Piece::owned(Color::Green, Size::Small, Player::One),
                        Piece::owned(Color::Red, Size::Small, Player::One),
                        Piece::owned(Color::Red, Size::Medium, Player::One),
                        Piece::owned(Color::Red, Size::Large, Player::Two),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![
                        Piece::new(Color::Yellow, Size::Small),
                        Piece::new(Color::Green, Size::Large),
                    ],
                    vec![
                        Piece::owned(Color::Blue, Size::Small, Player::Two),
                        Piece::owned(Color::Yellow, Size::Small, Player::Two),
                        Piece::owned(Color::Yellow, Size::Medium, Player::Two),
                        Piece::owned(Color::Yellow, Size::Large, Player::One),
                    ],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid");
        let turn = TurnState::from_parts(state, Player::One, 0, None);

        Game::from_parts(turn, GameStatus::InProgress)
    }

    fn duplicate_ship_game() -> Game {
        use hw_core::{Bank, GameState, StarSystem};

        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Green, Size::Small)],
                    vec![
                        Piece::owned(Color::Green, Size::Small, Player::One),
                        Piece::owned(Color::Green, Size::Small, Player::One),
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
