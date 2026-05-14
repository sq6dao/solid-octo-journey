use hw_engine::{Action, Game, GameStatus};

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
    decisions
}

fn push_end_turn_decision(game: &Game, decisions: &mut Vec<AiDecision>) {
    if game.end_turn().is_ok() {
        decisions.push(AiDecision::EndTurn);
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
}
