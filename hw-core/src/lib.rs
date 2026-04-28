#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Blue,
}

impl Color {
    pub const ALL: [Self; 4] = [Self::Red, Self::Yellow, Self::Green, Self::Blue];
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Player {
    One,
    Two,
}

impl Player {
    pub const ALL: [Self; 2] = [Self::One, Self::Two];
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Piece {
    color: Color,
    size: Size,
    owner: Option<Player>,
}

impl Piece {
    pub const fn new(color: Color, size: Size) -> Self {
        Self {
            color,
            size,
            owner: None,
        }
    }

    pub const fn owned(color: Color, size: Size, owner: Player) -> Self {
        Self {
            color,
            size,
            owner: Some(owner),
        }
    }

    pub const fn color(&self) -> Color {
        self.color
    }

    pub const fn size(&self) -> Size {
        self.size
    }

    pub const fn owner(&self) -> Option<Player> {
        self.owner
    }

    pub const fn is_owned(&self) -> bool {
        self.owner.is_some()
    }

    pub fn is_owned_by(&self, player: Player) -> bool {
        self.owner == Some(player)
    }

    pub const fn with_owner(self, owner: Player) -> Self {
        Self {
            owner: Some(owner),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_are_the_four_homeworlds_colors() {
        assert_eq!(
            Color::ALL,
            [Color::Red, Color::Yellow, Color::Green, Color::Blue]
        );
    }

    #[test]
    fn sizes_are_the_three_piece_sizes() {
        assert_eq!(Size::ALL, [Size::Small, Size::Medium, Size::Large]);
    }

    #[test]
    fn players_are_the_two_supported_seats() {
        assert_eq!(Player::ALL, [Player::One, Player::Two]);
    }

    #[test]
    fn piece_can_be_constructed_without_an_owner() {
        let piece = Piece::new(Color::Blue, Size::Large);

        assert_eq!(piece.color(), Color::Blue);
        assert_eq!(piece.size(), Size::Large);
        assert_eq!(piece.owner(), None);
        assert!(!piece.is_owned());
    }

    #[test]
    fn piece_can_be_constructed_with_a_player_owner() {
        let piece = Piece::owned(Color::Green, Size::Small, Player::Two);

        assert_eq!(piece.color(), Color::Green);
        assert_eq!(piece.size(), Size::Small);
        assert_eq!(piece.owner(), Some(Player::Two));
        assert!(piece.is_owned());
        assert!(piece.is_owned_by(Player::Two));
        assert!(!piece.is_owned_by(Player::One));
    }

    #[test]
    fn piece_owner_can_be_changed_without_changing_identity() {
        let piece = Piece::new(Color::Red, Size::Medium).with_owner(Player::One);

        assert_eq!(piece.color(), Color::Red);
        assert_eq!(piece.size(), Size::Medium);
        assert_eq!(piece.owner(), Some(Player::One));
    }
}
