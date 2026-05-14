mod common;

use common::*;
use hw_core::{Color, Player, Size};
use hw_engine::{Game, GameError, GameOutcome, GameStatus};

#[test]
fn sample_game_uses_build_travel_trade_and_invade() {
    let mut game = simple_game(Player::One);

    game = apply_and_end(
        &game,
        &build(Player::One, P1_HOME, Color::Yellow, Size::Small),
    );
    game = apply_and_end(&game, &build(Player::Two, P2_HOME, Color::Red, Size::Small));
    game = apply_and_end(
        &game,
        &build(Player::One, P1_HOME, Color::Blue, Size::Small),
    );
    game = apply_and_end(
        &game,
        &build(Player::Two, P2_HOME, Color::Yellow, Size::Small),
    );

    game = apply_and_end(
        &game,
        &trade(
            Player::One,
            P1_HOME,
            ship(Player::One, Color::Blue, Size::Small),
            ship(Player::One, Color::Red, Size::Small),
        ),
    );
    game = apply_and_end(
        &game,
        &build(Player::Two, P2_HOME, Color::Blue, Size::Small),
    );

    let traveler = ship(Player::One, Color::Yellow, Size::Small);
    game = apply_and_end(
        &game,
        &travel_existing(Player::One, P1_HOME, traveler, P2_HOME),
    );

    assert_eq!(count_ship(game.turn().state(), P1_HOME, traveler), 0);
    assert_eq!(count_ship(game.turn().state(), P2_HOME, traveler), 1);

    let after_invade = game
        .apply_action(&invade(Player::Two, P2_HOME, traveler))
        .expect("invade applies");

    assert_eq!(after_invade.status(), GameStatus::InProgress);
    assert_eq!(
        count_ship(
            after_invade.turn().state(),
            P2_HOME,
            ship(Player::Two, Color::Yellow, Size::Small),
        ),
        2
    );
    assert_eq!(
        count_ship(after_invade.turn().state(), P2_HOME, traveler),
        0
    );
}

#[test]
fn short_game_ends_when_a_catastrophe_purges_a_homeworld() {
    let finished = game_won_by_player_one();

    assert_eq!(
        finished.status(),
        GameStatus::Finished(GameOutcome::Winner(Player::One))
    );
}

#[test]
fn terminal_sample_game_rejects_more_actions_and_turn_ending() {
    let finished = game_won_by_player_one();
    let outcome = GameOutcome::Winner(Player::One);

    assert_eq!(
        finished.apply_action(&build(Player::Two, P2_HOME, Color::Green, Size::Small,)),
        Err(GameError::Terminal { outcome })
    );
    assert_eq!(finished.end_turn(), Err(GameError::Terminal { outcome }));
}

fn game_won_by_player_one() -> Game {
    let mut game = catastrophe_win_game();

    game = apply_and_end(
        &game,
        &build(Player::One, P1_HOME, Color::Yellow, Size::Small),
    );
    game = apply_and_end(&game, &build(Player::Two, P2_HOME, Color::Red, Size::Small));
    game = apply_and_end(
        &game,
        &build(Player::One, P1_HOME, Color::Blue, Size::Small),
    );
    game = apply_and_end(&game, &build(Player::Two, P2_HOME, Color::Red, Size::Small));

    game.apply_action(&catastrophe(P2_HOME, Color::Red))
        .expect("catastrophe applies")
        .apply_action(&build(Player::One, P1_HOME, Color::Green, Size::Small))
        .expect("build applies")
        .end_turn()
        .expect("turn ends")
}

fn catastrophe_win_game() -> Game {
    Game::new(
        [
            setup(
                vec![piece(Color::Yellow, Size::Large)],
                ship(Player::One, Color::Green, Size::Small),
            ),
            setup(
                vec![
                    piece(Color::Red, Size::Small),
                    piece(Color::Red, Size::Medium),
                ],
                ship(Player::Two, Color::Green, Size::Small),
            ),
        ],
        Player::One,
    )
    .expect("game initializes")
}
