use hw_core::{GameState, Player};

use crate::{Action, TransitionError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnState {
    state: GameState,
    current_player: Player,
    remaining_actions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TurnError {
    InvalidAction(TransitionError),
    WrongPlayer { expected: Player, actual: Player },
    NoActionBudget,
    ActionsRemaining { remaining: usize },
}

impl TurnState {
    pub const fn new(state: GameState, current_player: Player) -> Self {
        Self {
            state,
            current_player,
            remaining_actions: 1,
        }
    }

    pub const fn state(&self) -> &GameState {
        &self.state
    }

    pub const fn current_player(&self) -> Player {
        self.current_player
    }

    pub const fn remaining_actions(&self) -> usize {
        self.remaining_actions
    }

    pub fn apply_action(&self, action: &Action) -> Result<Self, TurnError> {
        if matches!(action, Action::Catastrophe { .. }) {
            let state = crate::apply_action(&self.state, action).map_err(TurnError::InvalidAction)?;

            return Ok(Self {
                state,
                current_player: self.current_player,
                remaining_actions: self.remaining_actions,
            });
        }

        if let Some(player) = action_player(action) {
            if player != self.current_player {
                return Err(TurnError::WrongPlayer {
                    expected: self.current_player,
                    actual: player,
                });
            }
        }

        if self.remaining_actions == 0 {
            return Err(TurnError::NoActionBudget);
        }

        let state = crate::apply_action(&self.state, action).map_err(TurnError::InvalidAction)?;

        Ok(Self {
            state,
            current_player: self.current_player,
            remaining_actions: self.remaining_actions - 1,
        })
    }

    pub fn end_turn(&self) -> Result<Self, TurnError> {
        if self.remaining_actions > 0 {
            return Err(TurnError::ActionsRemaining {
                remaining: self.remaining_actions,
            });
        }

        Ok(Self {
            state: self.state.clone(),
            current_player: other_player(self.current_player),
            remaining_actions: 1,
        })
    }
}

fn action_player(action: &Action) -> Option<Player> {
    match action {
        Action::Build { player, .. }
        | Action::Move { player, .. }
        | Action::Trade { player, .. }
        | Action::Sacrifice { player, .. }
        | Action::Invade { player, .. } => Some(*player),
        Action::Catastrophe { .. } => None,
    }
}

const fn other_player(player: Player) -> Player {
    match player {
        Player::One => Player::Two,
        Player::Two => Player::One,
    }
}

#[cfg(test)]
mod tests {
    use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, SystemId};

    use super::*;
    use crate::{Action, ActionError, TransitionError};

    #[test]
    fn turn_state_tracks_the_current_player_and_state() {
        let state = valid_state();

        let turn = TurnState::new(state.clone(), Player::One);

        assert_eq!(turn.current_player(), Player::One);
        assert_eq!(turn.remaining_actions(), 1);
        assert_eq!(turn.state(), &state);
    }

    #[test]
    fn applying_a_current_player_action_consumes_the_normal_budget() {
        let turn = TurnState::new(valid_state(), Player::One);
        let ship = owned_ship(Player::One, Color::Green, Size::Small);
        let action = build_action(Player::One, ship);

        let next = turn.apply_action(&action).expect("action applies");

        assert_eq!(next.current_player(), Player::One);
        assert_eq!(next.remaining_actions(), 0);
        assert_eq!(
            count_ship(next.state().system(SystemId::new(0)).expect("system exists"), ship),
            2
        );
    }

    #[test]
    fn applying_an_action_for_the_wrong_player_is_rejected() {
        let turn = TurnState::new(valid_state(), Player::One);
        let action = build_action(
            Player::Two,
            owned_ship(Player::Two, Color::Green, Size::Small),
        );

        assert_eq!(
            turn.apply_action(&action),
            Err(TurnError::WrongPlayer {
                expected: Player::One,
                actual: Player::Two,
            })
        );
    }

    #[test]
    fn applying_a_player_action_without_budget_is_rejected() {
        let turn = TurnState::new(valid_state(), Player::One);
        let first = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Small),
        );
        let second = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Medium),
        );

        let next = turn.apply_action(&first).expect("first action applies");

        assert_eq!(next.apply_action(&second), Err(TurnError::NoActionBudget));
    }

    #[test]
    fn end_turn_switches_player_after_budget_is_spent() {
        let turn = TurnState::new(valid_state(), Player::One);
        let action = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Small),
        );
        let spent = turn.apply_action(&action).expect("action applies");

        let next = spent.end_turn().expect("turn ends");

        assert_eq!(next.current_player(), Player::Two);
        assert_eq!(next.remaining_actions(), 1);
    }

    #[test]
    fn end_turn_rejects_turns_with_unspent_budget() {
        let turn = TurnState::new(valid_state(), Player::One);

        assert_eq!(
            turn.end_turn(),
            Err(TurnError::ActionsRemaining { remaining: 1 })
        );
    }

    #[test]
    fn action_validation_errors_are_preserved() {
        let mut bank = Bank::new();
        for _ in 0..Bank::copies_per_piece() {
            bank.draw(Color::Green, Size::Small).expect("piece exists");
        }
        let turn = TurnState::new(state_with_bank(bank), Player::One);
        let ship = owned_ship(Player::One, Color::Green, Size::Small);
        let action = build_action(Player::One, ship);

        assert_eq!(
            turn.apply_action(&action),
            Err(TurnError::InvalidAction(TransitionError::InvalidAction(
                ActionError::PieceUnavailable { piece: ship }
            )))
        );
    }

    #[test]
    fn catastrophe_actions_do_not_consume_budget() {
        let turn = TurnState::new(state_with_catastrophe(), Player::One);
        let action = Action::Catastrophe {
            system: SystemId::new(0),
            color: Color::Red,
        };

        let next = turn.apply_action(&action).expect("catastrophe applies");

        assert_eq!(next.current_player(), Player::One);
        assert_eq!(next.remaining_actions(), 1);
        assert!(
            next.state()
                .system(SystemId::new(0))
                .expect("system exists")
                .stars()
                .iter()
                .all(|piece| piece.color() != Color::Red)
        );
        assert!(
            next.state()
                .system(SystemId::new(0))
                .expect("system exists")
                .ships()
                .iter()
                .all(|piece| piece.color() != Color::Red)
        );
    }

    #[test]
    fn catastrophe_actions_can_apply_after_budget_is_spent() {
        let turn = TurnState::new(state_with_catastrophe(), Player::One);
        let build = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Small),
        );
        let catastrophe = Action::Catastrophe {
            system: SystemId::new(0),
            color: Color::Red,
        };
        let spent = turn.apply_action(&build).expect("build applies");

        let next = spent
            .apply_action(&catastrophe)
            .expect("catastrophe applies");

        assert_eq!(next.current_player(), Player::One);
        assert_eq!(next.remaining_actions(), 0);
    }

    #[test]
    fn end_turn_allows_unresolved_catastrophes() {
        let turn = TurnState::new(state_with_catastrophe(), Player::One);
        let action = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Small),
        );
        let spent = turn.apply_action(&action).expect("action applies");

        let next = spent.end_turn().expect("turn ends");

        assert_eq!(next.current_player(), Player::Two);
        assert_eq!(next.remaining_actions(), 1);
    }

    fn build_action(player: Player, ship: Piece) -> Action {
        Action::Build {
            player,
            system: SystemId::new(0),
            ship,
        }
    }

    fn valid_state() -> GameState {
        state_with_bank(Bank::new())
    }

    fn state_with_catastrophe() -> GameState {
        GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Red, Size::Medium),
                    ],
                    vec![
                        owned_ship(Player::One, Color::Red, Size::Small),
                        owned_ship(Player::Two, Color::Red, Size::Large),
                        owned_ship(Player::One, Color::Green, Size::Small),
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
            Bank::new(),
        )
        .expect("state is valid")
    }

    fn state_with_bank(bank: Bank) -> GameState {
        GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Small)],
                    vec![
                        owned_ship(Player::One, Color::Blue, Size::Small),
                        owned_ship(Player::One, Color::Green, Size::Small),
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

    fn owned_ship(player: Player, color: Color, size: Size) -> Piece {
        Piece::owned(color, size, player)
    }

    fn count_ship(system: &StarSystem, ship: Piece) -> usize {
        system
            .ships()
            .iter()
            .filter(|candidate| **candidate == ship)
            .count()
    }
}
