use crate::{Piece, Player};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StarSystemError {
    TooManyStars,
    OwnedStar,
    NoShips,
    UnownedShip,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StarSystem {
    stars: Vec<Piece>,
    ships: Vec<Piece>,
}

impl StarSystem {
    pub fn new(stars: Vec<Piece>, ships: Vec<Piece>) -> Result<Self, StarSystemError> {
        if stars.len() > 2 {
            return Err(StarSystemError::TooManyStars);
        }

        if stars.iter().any(Piece::is_owned) {
            return Err(StarSystemError::OwnedStar);
        }

        if ships.is_empty() {
            return Err(StarSystemError::NoShips);
        }

        if ships.iter().any(|ship| !ship.is_owned()) {
            return Err(StarSystemError::UnownedShip);
        }

        Ok(Self { stars, ships })
    }

    pub fn stars(&self) -> &[Piece] {
        &self.stars
    }

    pub fn ships(&self) -> &[Piece] {
        &self.ships
    }

    pub fn has_presence(&self, player: Player) -> bool {
        self.ships.iter().any(|ship| ship.is_owned_by(player))
    }

    pub fn owners_present(&self) -> Vec<Player> {
        Player::ALL
            .into_iter()
            .filter(|player| self.has_presence(*player))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Size};

    #[test]
    fn star_system_can_be_constructed_with_one_star_and_one_ship() {
        let star = Piece::new(Color::Yellow, Size::Medium);
        let ship = Piece::owned(Color::Blue, Size::Small, Player::One);

        let system = StarSystem::new(vec![star], vec![ship]).expect("system is valid");

        assert_eq!(system.stars(), &[star]);
        assert_eq!(system.ships(), &[ship]);
    }

    #[test]
    fn star_system_can_be_constructed_with_two_stars() {
        let stars = vec![
            Piece::new(Color::Yellow, Size::Medium),
            Piece::new(Color::Blue, Size::Large),
        ];
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];

        let system = StarSystem::new(stars.clone(), ships).expect("system is valid");

        assert_eq!(system.stars(), stars.as_slice());
    }

    #[test]
    fn star_system_can_be_constructed_with_zero_stars() {
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];
        let system = StarSystem::new(vec![], ships).expect("system is valid");

        assert!(system.stars().is_empty());
    }

    #[test]
    fn star_system_rejects_more_than_two_stars() {
        let stars = vec![
            Piece::new(Color::Yellow, Size::Small),
            Piece::new(Color::Blue, Size::Medium),
            Piece::new(Color::Red, Size::Large),
        ];
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];

        assert_eq!(
            StarSystem::new(stars, ships),
            Err(StarSystemError::TooManyStars)
        );
    }

    #[test]
    fn star_system_rejects_owned_stars() {
        let stars = vec![Piece::owned(Color::Yellow, Size::Small, Player::One)];
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];

        assert_eq!(
            StarSystem::new(stars, ships),
            Err(StarSystemError::OwnedStar)
        );
    }

    #[test]
    fn star_system_rejects_zero_ships() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];

        assert_eq!(
            StarSystem::new(stars, vec![]),
            Err(StarSystemError::NoShips)
        );
    }

    #[test]
    fn star_system_rejects_unowned_ships() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];
        let ships = vec![Piece::new(Color::Green, Size::Small)];

        assert_eq!(
            StarSystem::new(stars, ships),
            Err(StarSystemError::UnownedShip)
        );
    }

    #[test]
    fn star_system_detects_player_presence() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];
        let ships = vec![Piece::owned(Color::Green, Size::Small, Player::One)];
        let system = StarSystem::new(stars, ships).expect("system is valid");

        assert!(system.has_presence(Player::One));
        assert!(!system.has_presence(Player::Two));
    }

    #[test]
    fn star_system_reports_both_players_when_both_have_ships() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];
        let ships = vec![
            Piece::owned(Color::Green, Size::Small, Player::Two),
            Piece::owned(Color::Blue, Size::Large, Player::One),
        ];
        let system = StarSystem::new(stars, ships).expect("system is valid");

        assert_eq!(system.owners_present(), vec![Player::One, Player::Two]);
    }

    #[test]
    fn star_system_reports_each_player_once() {
        let stars = vec![Piece::new(Color::Yellow, Size::Small)];
        let ships = vec![
            Piece::owned(Color::Green, Size::Small, Player::One),
            Piece::owned(Color::Blue, Size::Large, Player::One),
        ];
        let system = StarSystem::new(stars, ships).expect("system is valid");

        assert_eq!(system.owners_present(), vec![Player::One]);
    }
}
