use hw_core::Player;
use hw_engine::{Game, GameStatus};

use crate::{AiDecision, Strategy, evaluate_position, legal_decisions};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScoredDecision {
    pub decision: AiDecision,
    pub score: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchStrategy {
    depth: usize,
}

impl SearchStrategy {
    pub const fn new(depth: usize) -> Self {
        Self { depth }
    }

    pub const fn depth(self) -> usize {
        self.depth
    }
}

impl Default for SearchStrategy {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Strategy for SearchStrategy {
    fn choose(&self, game: &Game) -> Option<AiDecision> {
        best_decision_at_depth(game, self.depth).map(|scored| scored.decision)
    }
}

pub fn best_decision(game: &Game) -> Option<ScoredDecision> {
    best_decision_at_depth(game, 1)
}

pub fn best_decision_at_depth(game: &Game, depth: usize) -> Option<ScoredDecision> {
    if depth == 0 || game.status() != GameStatus::InProgress {
        return None;
    }

    let root_player = game.turn().current_player();
    best_at_node(game, root_player, depth)
}

fn best_at_node(game: &Game, root_player: Player, depth: usize) -> Option<ScoredDecision> {
    if depth == 0 || game.status() != GameStatus::InProgress {
        return None;
    }

    let maximizing = game.turn().current_player() == root_player;
    let mut best = None;

    for decision in legal_decisions(game) {
        if crate::is_immediate_non_win(game, &decision) {
            continue;
        }

        let Some(next) = apply_decision(game, &decision) else {
            continue;
        };
        let score = search_score(&next, root_player, depth - 1);
        let scored = ScoredDecision { decision, score };

        if should_replace(best.as_ref(), &scored, maximizing) {
            best = Some(scored);
        }
    }

    best
}

fn search_score(game: &Game, root_player: Player, depth: usize) -> i32 {
    if depth == 0 || game.status() != GameStatus::InProgress {
        return evaluate_position(game, root_player).total;
    }

    best_at_node(game, root_player, depth)
        .map(|scored| scored.score)
        .unwrap_or_else(|| evaluate_position(game, root_player).total)
}

fn should_replace(
    current: Option<&ScoredDecision>,
    candidate: &ScoredDecision,
    maximizing: bool,
) -> bool {
    match current {
        None => true,
        Some(current) if maximizing => candidate.score > current.score,
        Some(current) => candidate.score < current.score,
    }
}

pub(super) fn apply_decision(game: &Game, decision: &AiDecision) -> Option<Game> {
    match decision {
        AiDecision::Action(action) => game.apply_action(action).ok(),
        AiDecision::EndTurn => game.end_turn().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AiDecision, Strategy, evaluate_position, legal_decisions};
    use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, SystemId};
    use hw_engine::{Game, GameOutcome, GameStatus, TurnState};

    #[test]
    fn one_ply_search_chooses_highest_immediate_evaluation() {
        let game = Game::default(Player::One);
        let selected = best_decision(&game).expect("search selects a decision");
        let expected = first_highest_one_ply_decision(&game);

        assert_eq!(selected, expected);
    }

    #[test]
    fn depth_limited_search_scores_visible_opponent_reply() {
        let game = opponent_reply_game();
        let one_ply = best_decision_at_depth(&game, 1).expect("one-ply decision");
        let deeper = best_decision_at_depth(&game, 2).expect("deeper decision");

        assert_eq!(one_ply.decision, AiDecision::EndTurn);
        assert_eq!(deeper.decision, AiDecision::EndTurn);
        assert!(deeper.score < one_ply.score);
    }

    #[test]
    fn zero_depth_search_does_not_select_a_decision() {
        assert_eq!(best_decision_at_depth(&Game::default(Player::One), 0), None);
    }

    #[test]
    fn equal_scores_preserve_legal_decision_order() {
        let game = equal_score_travel_game();
        let selected = best_decision(&game).expect("search selects a decision");
        let tied_decisions = legal_decisions(&game)
            .into_iter()
            .filter(|decision| !crate::is_immediate_non_win(&game, decision))
            .filter_map(|decision| {
                super::apply_decision(&game, &decision).map(|next| ScoredDecision {
                    decision,
                    score: evaluate_position(&next, Player::One).total,
                })
            })
            .filter(|scored| scored.score == selected.score)
            .collect::<Vec<_>>();

        assert!(tied_decisions.len() > 1);
        assert_eq!(selected, tied_decisions[0]);
    }

    #[test]
    fn search_returns_none_for_terminal_games() {
        let game = Game::from_parts(
            Game::default(Player::One).turn().clone(),
            GameStatus::Finished(GameOutcome::Winner(Player::One)),
        );

        assert_eq!(best_decision(&game), None);
    }

    #[test]
    fn search_returns_none_when_all_decisions_are_immediately_unsafe() {
        let game = empty_current_homeworld_game();

        assert_eq!(best_decision(&game), None);
    }

    #[test]
    fn search_strategy_uses_configured_depth() {
        let game = opponent_reply_game();

        assert_eq!(
            SearchStrategy::new(2).choose(&game),
            best_decision_at_depth(&game, 2).map(|scored| scored.decision)
        );
    }

    fn first_highest_one_ply_decision(game: &Game) -> ScoredDecision {
        let player = game.turn().current_player();
        let mut best = None;

        for decision in legal_decisions(game) {
            if crate::is_immediate_non_win(game, &decision) {
                continue;
            }

            let Some(next) = super::apply_decision(game, &decision) else {
                continue;
            };
            let scored = ScoredDecision {
                decision,
                score: evaluate_position(&next, player).total,
            };

            if best
                .as_ref()
                .is_none_or(|current: &ScoredDecision| scored.score > current.score)
            {
                best = Some(scored);
            }
        }

        best.expect("there is a legal safe decision")
    }

    fn opponent_reply_game() -> Game {
        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Small)],
                    vec![Piece::owned(Color::Green, Size::Small, Player::One)],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Red, Size::Medium)],
                    vec![Piece::owned(Color::Blue, Size::Large, Player::Two)],
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

    fn equal_score_travel_game() -> Game {
        let empty_bank =
            Bank::from_counts([[0; Size::COUNT]; Color::COUNT]).expect("counts are valid");
        let state = GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    vec![
                        Piece::owned(Color::Yellow, Size::Small, Player::One),
                        Piece::owned(Color::Yellow, Size::Medium, Player::One),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(vec![Piece::new(Color::Green, Size::Large)], vec![])
                    .expect("system is valid"),
                StarSystem::new(vec![Piece::new(Color::Green, Size::Large)], vec![])
                    .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Large)],
                    vec![Piece::owned(Color::Red, Size::Small, Player::Two)],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(3)],
            empty_bank,
        )
        .expect("state is valid");

        Game::from_parts(
            TurnState::from_parts(state, Player::One, 1, None),
            GameStatus::InProgress,
        )
    }

    fn empty_current_homeworld_game() -> Game {
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
}
