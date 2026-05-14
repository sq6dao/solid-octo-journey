use hw_core::{Color, Piece, Size, SystemId};
use hw_engine::{Action, ActionKind, Game, GameStatus};

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
    use hw_engine::{GameOutcome, TurnState};

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
        assert_eq!(
            action_kinds(&decisions),
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

        assert!(legal_decisions(&game).is_empty());
    }

    fn assert_all_actions_apply(game: &Game, decisions: &[AiDecision]) {
        for decision in decisions {
            if let AiDecision::Action(action) = decision {
                game.apply_action(action)
                    .expect("generated action should apply");
            }
        }
    }

    fn action_kinds(decisions: &[AiDecision]) -> Vec<ActionKind> {
        decisions
            .iter()
            .filter_map(|decision| match decision {
                AiDecision::Action(action) => Some(action.kind()),
                AiDecision::EndTurn => None,
            })
            .collect()
    }
}
