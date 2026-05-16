use hw_core::{Color, GameState, Player, Size};

use crate::{Action, ActionError, ActionKind, TransitionError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnState {
    state: GameState,
    current_player: Player,
    remaining_actions: usize,
    required_action: Option<ActionKind>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TurnError {
    InvalidAction(TransitionError),
    WrongPlayer {
        expected: Player,
        actual: Player,
    },
    NoActionBudget,
    ActionsRemaining {
        remaining: usize,
    },
    WrongSacrificeActionKind {
        expected: ActionKind,
        actual: ActionKind,
    },
}

impl TurnState {
    pub const fn new(state: GameState, current_player: Player) -> Self {
        Self {
            state,
            current_player,
            remaining_actions: 1,
            required_action: None,
        }
    }

    pub const fn from_parts(
        state: GameState,
        current_player: Player,
        remaining_actions: usize,
        required_action: Option<ActionKind>,
    ) -> Self {
        Self {
            state,
            current_player,
            remaining_actions,
            required_action,
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

    pub const fn required_action(&self) -> Option<ActionKind> {
        self.required_action
    }

    pub fn apply_action(&self, action: &Action) -> Result<Self, TurnError> {
        if matches!(action, Action::Catastrophe { .. }) {
            let state =
                crate::apply_action(&self.state, action).map_err(TurnError::InvalidAction)?;

            return Ok(Self {
                state,
                current_player: self.current_player,
                remaining_actions: self.remaining_actions,
                required_action: self.required_action,
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

        if let Some(expected) = self.required_action {
            let actual = action.kind();
            if actual != expected {
                return Err(TurnError::WrongSacrificeActionKind { expected, actual });
            }
        }

        self.validate_paid_action(action)?;
        let state = crate::transition::apply_action_unchecked(&self.state, action)
            .map_err(TurnError::InvalidAction)?;

        if let Action::Sacrifice { ship, .. } = action {
            return Ok(Self {
                state,
                current_player: self.current_player,
                remaining_actions: sacrifice_budget(ship.size()),
                required_action: Some(sacrifice_action_kind(ship.color())),
            });
        }

        let remaining_actions = self.remaining_actions - 1;

        Ok(Self {
            state,
            current_player: self.current_player,
            remaining_actions,
            required_action: if remaining_actions == 0 {
                None
            } else {
                self.required_action
            },
        })
    }

    pub fn end_turn(&self) -> Result<Self, TurnError> {
        if self.remaining_actions > 0 && self.required_action.is_none() {
            return Err(TurnError::ActionsRemaining {
                remaining: self.remaining_actions,
            });
        }

        Ok(Self {
            state: self.state.clone(),
            current_player: other_player(self.current_player),
            remaining_actions: 1,
            required_action: None,
        })
    }

    fn validate_paid_action(&self, action: &Action) -> Result<(), TurnError> {
        match crate::validate_action(&self.state, action) {
            Ok(()) => Ok(()),
            Err(error) if self.sacrifice_power_covers(action, &error) => Ok(()),
            Err(error) => Err(TurnError::InvalidAction(TransitionError::InvalidAction(
                error,
            ))),
        }
    }

    fn sacrifice_power_covers(&self, action: &Action, error: &ActionError) -> bool {
        let Some(expected) = self.required_action else {
            return false;
        };
        if action.kind() != expected {
            return false;
        }
        let Some(expected_color) = action_kind_color(expected) else {
            return false;
        };

        matches!(
            error,
            ActionError::MissingActionPower { color, .. } if *color == expected_color
        )
    }
}

fn action_player(action: &Action) -> Option<Player> {
    match action {
        Action::Build { player, .. }
        | Action::Travel { player, .. }
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

const fn sacrifice_budget(size: Size) -> usize {
    match size {
        Size::Small => 1,
        Size::Medium => 2,
        Size::Large => 3,
    }
}

const fn sacrifice_action_kind(color: Color) -> ActionKind {
    match color {
        Color::Green => ActionKind::Build,
        Color::Yellow => ActionKind::Travel,
        Color::Blue => ActionKind::Trade,
        Color::Red => ActionKind::Invade,
    }
}

const fn action_kind_color(kind: ActionKind) -> Option<Color> {
    match kind {
        ActionKind::Build => Some(Color::Green),
        ActionKind::Travel => Some(Color::Yellow),
        ActionKind::Trade => Some(Color::Blue),
        ActionKind::Invade => Some(Color::Red),
        ActionKind::Sacrifice | ActionKind::Catastrophe => None,
    }
}

#[cfg(test)]
mod tests {
    use hw_core::{Bank, Color, GameState, Piece, Player, Size, StarSystem, SystemId};

    use super::*;
    use crate::{Action, ActionError, TransitionError, TravelTarget};

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
            count_ship(
                next.state()
                    .system(SystemId::new(0))
                    .expect("system exists"),
                ship
            ),
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

    #[test]
    fn sacrificing_a_small_green_ship_grants_one_build_action() {
        let sacrifice_ship = owned_ship(Player::One, Color::Green, Size::Small);
        let turn = TurnState::new(state_with_green_sacrifice_ship(Size::Small), Player::One);
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let build = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Small),
        );

        let sacrificed = turn.apply_action(&sacrifice).expect("sacrifice applies");
        let built = sacrificed.apply_action(&build).expect("build applies");

        assert_eq!(sacrificed.remaining_actions(), 1);
        assert_eq!(built.remaining_actions(), 0);
    }

    #[test]
    fn sacrificing_a_medium_blue_ship_grants_two_trade_actions() {
        let sacrifice_ship = owned_ship(Player::One, Color::Blue, Size::Medium);
        let turn = TurnState::new(state_with_blue_sacrifice_fleet(), Player::One);
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let trade_small = trade_action(
            Player::One,
            owned_ship(Player::One, Color::Blue, Size::Small),
            owned_ship(Player::One, Color::Red, Size::Small),
        );
        let trade_large = trade_action(
            Player::One,
            owned_ship(Player::One, Color::Blue, Size::Large),
            owned_ship(Player::One, Color::Red, Size::Large),
        );

        let sacrificed = turn.apply_action(&sacrifice).expect("sacrifice applies");
        let traded_once = sacrificed
            .apply_action(&trade_small)
            .expect("first trade applies");
        let traded_twice = traded_once
            .apply_action(&trade_large)
            .expect("second trade applies");

        assert_eq!(sacrificed.remaining_actions(), 2);
        assert_eq!(traded_once.remaining_actions(), 1);
        assert_eq!(traded_twice.remaining_actions(), 0);
    }

    #[test]
    fn sacrificing_a_large_green_ship_grants_three_build_actions() {
        let sacrifice_ship = owned_ship(Player::One, Color::Green, Size::Large);
        let turn = TurnState::new(state_with_green_sacrifice_ship(Size::Large), Player::One);
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let build = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Small),
        );

        let after_sacrifice = turn.apply_action(&sacrifice).expect("sacrifice applies");
        let after_first_build = after_sacrifice
            .apply_action(&build)
            .expect("first build applies");
        let after_second_build = after_first_build
            .apply_action(&build)
            .expect("second build applies");
        let after_third_build = after_second_build
            .apply_action(&build)
            .expect("third build applies");

        assert_eq!(after_sacrifice.remaining_actions(), 3);
        assert_eq!(after_first_build.remaining_actions(), 2);
        assert_eq!(after_second_build.remaining_actions(), 1);
        assert_eq!(after_third_build.remaining_actions(), 0);
    }

    #[test]
    fn sacrifice_travel_actions_do_not_require_yellow_power_at_source() {
        let sacrifice_ship = owned_ship(Player::One, Color::Yellow, Size::Medium);
        let moving_ship = owned_ship(Player::One, Color::Green, Size::Small);
        let turn = TurnState::new(
            state_for_yellow_sacrifice_without_local_power(),
            Player::One,
        );
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let travel = Action::Travel {
            player: Player::One,
            from: SystemId::new(0),
            ship: moving_ship,
            target: TravelTarget::Existing(SystemId::new(1)),
        };

        let traveled = turn
            .apply_action(&sacrifice)
            .expect("sacrifice applies")
            .apply_action(&travel)
            .expect("travel applies without local yellow power");

        assert_eq!(traveled.remaining_actions(), 1);
        assert!(
            traveled
                .state()
                .system(SystemId::new(1))
                .expect("target exists")
                .ships()
                .contains(&moving_ship)
        );
    }

    #[test]
    fn sacrifice_trade_actions_do_not_require_blue_power_at_system() {
        let sacrifice_ship = owned_ship(Player::One, Color::Blue, Size::Medium);
        let from = owned_ship(Player::One, Color::Green, Size::Small);
        let to = owned_ship(Player::One, Color::Red, Size::Small);
        let turn = TurnState::new(state_for_blue_sacrifice_without_local_power(), Player::One);
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let trade = Action::Trade {
            player: Player::One,
            system: SystemId::new(1),
            from,
            to,
        };

        let traded = turn
            .apply_action(&sacrifice)
            .expect("sacrifice applies")
            .apply_action(&trade)
            .expect("trade applies without local blue power");

        assert_eq!(traded.remaining_actions(), 1);
        assert!(
            traded
                .state()
                .system(SystemId::new(1))
                .expect("trade system exists")
                .ships()
                .contains(&to)
        );
    }

    #[test]
    fn sacrifice_turns_reject_nonmatching_action_kinds() {
        let sacrifice_ship = owned_ship(Player::One, Color::Green, Size::Small);
        let turn = TurnState::new(state_with_green_sacrifice_ship(Size::Small), Player::One);
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let trade = trade_action(
            Player::One,
            owned_ship(Player::One, Color::Blue, Size::Small),
            owned_ship(Player::One, Color::Red, Size::Small),
        );
        let sacrificed = turn.apply_action(&sacrifice).expect("sacrifice applies");

        assert_eq!(
            sacrificed.apply_action(&trade),
            Err(TurnError::WrongSacrificeActionKind {
                expected: ActionKind::Build,
                actual: ActionKind::Trade,
            })
        );
    }

    #[test]
    fn catastrophe_actions_do_not_consume_sacrifice_budget() {
        let sacrifice_ship = owned_ship(Player::One, Color::Green, Size::Small);
        let turn = TurnState::new(state_with_catastrophe_sacrifice(), Player::One);
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let catastrophe = Action::Catastrophe {
            system: SystemId::new(0),
            color: Color::Red,
        };
        let build = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Small),
        );

        let sacrificed = turn.apply_action(&sacrifice).expect("sacrifice applies");
        let after_catastrophe = sacrificed
            .apply_action(&catastrophe)
            .expect("catastrophe applies");
        let built = after_catastrophe
            .apply_action(&build)
            .expect("build applies");

        assert_eq!(after_catastrophe.remaining_actions(), 1);
        assert_eq!(built.remaining_actions(), 0);
    }

    #[test]
    fn ending_a_sacrifice_turn_resets_the_action_kind_limit() {
        let sacrifice_ship = owned_ship(Player::One, Color::Green, Size::Small);
        let turn = TurnState::new(state_with_green_sacrifice_ship(Size::Small), Player::One);
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let build = build_action(
            Player::One,
            owned_ship(Player::One, Color::Green, Size::Small),
        );
        let next_player_travel = Action::Travel {
            player: Player::Two,
            from: SystemId::new(1),
            ship: owned_ship(Player::Two, Color::Yellow, Size::Small),
            target: TravelTarget::Existing(SystemId::new(0)),
        };
        let spent = turn
            .apply_action(&sacrifice)
            .expect("sacrifice applies")
            .apply_action(&build)
            .expect("build applies");

        let next_turn = spent.end_turn().expect("turn ends");
        let traveled = next_turn
            .apply_action(&next_player_travel)
            .expect("travel applies");

        assert_eq!(traveled.current_player(), Player::Two);
        assert_eq!(traveled.remaining_actions(), 0);
    }

    #[test]
    fn ending_a_sacrifice_turn_allows_unspent_granted_actions() {
        let sacrifice_ship = owned_ship(Player::One, Color::Green, Size::Large);
        let turn = TurnState::new(state_with_green_sacrifice_ship(Size::Large), Player::One);
        let sacrifice = sacrifice_action(Player::One, sacrifice_ship);
        let sacrificed = turn.apply_action(&sacrifice).expect("sacrifice applies");

        let next_turn = sacrificed.end_turn().expect("sacrifice turn ends early");

        assert_eq!(next_turn.current_player(), Player::Two);
        assert_eq!(next_turn.remaining_actions(), 1);
        assert_eq!(next_turn.required_action(), None);
    }

    fn build_action(player: Player, ship: Piece) -> Action {
        Action::Build {
            player,
            system: SystemId::new(0),
            ship,
        }
    }

    fn sacrifice_action(player: Player, ship: Piece) -> Action {
        Action::Sacrifice {
            player,
            system: SystemId::new(0),
            ship,
        }
    }

    fn trade_action(player: Player, from: Piece, to: Piece) -> Action {
        Action::Trade {
            player,
            system: SystemId::new(0),
            from,
            to,
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

    fn state_with_green_sacrifice_ship(size: Size) -> GameState {
        GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Small)],
                    vec![
                        owned_ship(Player::One, Color::Green, size),
                        owned_ship(Player::One, Color::Green, Size::Medium),
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

    fn state_with_blue_sacrifice_fleet() -> GameState {
        GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Small)],
                    vec![
                        owned_ship(Player::One, Color::Blue, Size::Small),
                        owned_ship(Player::One, Color::Blue, Size::Medium),
                        owned_ship(Player::One, Color::Blue, Size::Large),
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

    fn state_for_yellow_sacrifice_without_local_power() -> GameState {
        GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Red, Size::Small)],
                    vec![
                        owned_ship(Player::One, Color::Yellow, Size::Medium),
                        owned_ship(Player::One, Color::Green, Size::Small),
                    ],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Green, Size::Medium)],
                    vec![owned_ship(Player::Two, Color::Red, Size::Small)],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid")
    }

    fn state_for_blue_sacrifice_without_local_power() -> GameState {
        GameState::new(
            vec![
                StarSystem::new(
                    vec![Piece::new(Color::Red, Size::Small)],
                    vec![owned_ship(Player::One, Color::Blue, Size::Medium)],
                )
                .expect("system is valid"),
                StarSystem::new(
                    vec![Piece::new(Color::Yellow, Size::Medium)],
                    vec![owned_ship(Player::One, Color::Green, Size::Small)],
                )
                .expect("system is valid"),
            ],
            [SystemId::new(0), SystemId::new(1)],
            Bank::new(),
        )
        .expect("state is valid")
    }

    fn state_with_catastrophe_sacrifice() -> GameState {
        GameState::new(
            vec![
                StarSystem::new(
                    vec![
                        Piece::new(Color::Red, Size::Small),
                        Piece::new(Color::Blue, Size::Medium),
                    ],
                    vec![
                        owned_ship(Player::One, Color::Red, Size::Small),
                        owned_ship(Player::Two, Color::Red, Size::Medium),
                        owned_ship(Player::Two, Color::Red, Size::Large),
                        owned_ship(Player::One, Color::Green, Size::Small),
                        owned_ship(Player::One, Color::Green, Size::Medium),
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
