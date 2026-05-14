mod common;

use common::*;
use hw_core::{Color, Player, Size, SystemId};
use hw_engine::{GameError, GameStatus, TurnError, TurnState};

#[test]
fn normal_paid_action_consumes_budget_and_end_turn_switches_player() {
    let game = simple_game(Player::One);
    let built_ship = ship(Player::One, Color::Green, Size::Small);

    let after_action = game
        .apply_action(&build(Player::One, P1_HOME, Color::Green, Size::Small))
        .expect("build applies");

    assert_eq!(after_action.status(), GameStatus::InProgress);
    assert_eq!(after_action.turn().current_player(), Player::One);
    assert_eq!(after_action.turn().remaining_actions(), 0);
    assert_eq!(
        count_ship(after_action.turn().state(), P1_HOME, built_ship),
        2
    );

    let after_turn = after_action.end_turn().expect("turn ends");

    assert_eq!(after_turn.status(), GameStatus::InProgress);
    assert_eq!(after_turn.turn().current_player(), Player::Two);
    assert_eq!(after_turn.turn().remaining_actions(), 1);
}

#[test]
fn player_cannot_act_out_of_turn() {
    let game = simple_game(Player::One);

    assert_eq!(
        game.apply_action(&build(Player::Two, P2_HOME, Color::Green, Size::Small,)),
        Err(GameError::Turn(TurnError::WrongPlayer {
            expected: Player::One,
            actual: Player::Two,
        }))
    );
}

#[test]
fn catastrophe_costs_zero_actions_and_does_not_block_turn_end() {
    let turn = TurnState::new(state_with_red_catastrophe(), Player::One);

    let after_catastrophe = turn
        .apply_action(&catastrophe(P1_HOME, Color::Red))
        .expect("catastrophe applies");

    assert_eq!(after_catastrophe.current_player(), Player::One);
    assert_eq!(after_catastrophe.remaining_actions(), 1);
    assert_eq!(
        count_ship(
            after_catastrophe.state(),
            P1_HOME,
            ship(Player::One, Color::Green, Size::Small),
        ),
        1
    );

    let after_action = after_catastrophe
        .apply_action(&build(Player::One, P1_HOME, Color::Green, Size::Small))
        .expect("build applies");
    let after_turn = after_action.end_turn().expect("turn ends");

    assert_eq!(after_turn.current_player(), Player::Two);
    assert_eq!(after_turn.remaining_actions(), 1);
}

#[test]
fn sacrifice_turn_grants_size_based_same_color_actions_then_resets() {
    let turn = TurnState::new(state_with_green_sacrifice_fleet(), Player::One);

    let after_sacrifice = turn
        .apply_action(&sacrifice(
            Player::One,
            P1_HOME,
            ship(Player::One, Color::Green, Size::Medium),
        ))
        .expect("sacrifice applies");

    assert_eq!(after_sacrifice.remaining_actions(), 2);

    let after_first_build = after_sacrifice
        .apply_action(&build(Player::One, P1_HOME, Color::Yellow, Size::Small))
        .expect("first build applies");
    let after_second_build = after_first_build
        .apply_action(&build(Player::One, P1_HOME, Color::Blue, Size::Small))
        .expect("second build applies");

    assert_eq!(after_second_build.remaining_actions(), 0);

    let next_turn = after_second_build.end_turn().expect("turn ends");

    assert_eq!(next_turn.current_player(), Player::Two);
    assert_eq!(next_turn.remaining_actions(), 1);

    let after_player_two_build = next_turn
        .apply_action(&build(Player::Two, P2_HOME, Color::Green, Size::Small))
        .expect("next player's unrestricted action applies");

    assert_eq!(after_player_two_build.remaining_actions(), 0);
}

fn state_with_red_catastrophe() -> hw_core::GameState {
    state(
        vec![
            system(
                vec![
                    piece(Color::Red, Size::Small),
                    piece(Color::Blue, Size::Medium),
                ],
                vec![
                    ship(Player::One, Color::Green, Size::Small),
                    ship(Player::One, Color::Red, Size::Small),
                    ship(Player::Two, Color::Red, Size::Medium),
                    ship(Player::Two, Color::Red, Size::Large),
                ],
            ),
            system(
                vec![piece(Color::Yellow, Size::Large)],
                vec![ship(Player::Two, Color::Green, Size::Small)],
            ),
        ],
        [P1_HOME, P2_HOME],
    )
}

fn state_with_green_sacrifice_fleet() -> hw_core::GameState {
    state(
        vec![
            system(
                vec![piece(Color::Yellow, Size::Small)],
                vec![
                    ship(Player::One, Color::Green, Size::Medium),
                    ship(Player::One, Color::Green, Size::Small),
                ],
            ),
            system(
                vec![piece(Color::Blue, Size::Large)],
                vec![ship(Player::Two, Color::Green, Size::Small)],
            ),
        ],
        [SystemId::new(0), SystemId::new(1)],
    )
}
