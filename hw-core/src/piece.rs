use crate::{Color, Player, Size};

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
