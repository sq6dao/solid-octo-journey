#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Color {
    Red,
    Yellow,
    Green,
    Blue,
}

impl Color {
    pub const COUNT: usize = 4;
    pub const ALL: [Self; Self::COUNT] = [Self::Red, Self::Yellow, Self::Green, Self::Blue];

    const fn index(self) -> usize {
        match self {
            Self::Red => 0,
            Self::Yellow => 1,
            Self::Green => 2,
            Self::Blue => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    pub const COUNT: usize = 3;
    pub const ALL: [Self; Self::COUNT] = [Self::Small, Self::Medium, Self::Large];

    const fn index(self) -> usize {
        match self {
            Self::Small => 0,
            Self::Medium => 1,
            Self::Large => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Player {
    One,
    Two,
}

impl Player {
    pub const COUNT: usize = 2;
    pub const ALL: [Self; Self::COUNT] = [Self::One, Self::Two];
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BankError {
    PieceUnavailable,
    OwnedPiece,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bank {
    counts: [[u8; Size::COUNT]; Color::COUNT],
}

impl Bank {
    pub fn new() -> Self {
        Self {
            counts: [[Self::copies_per_piece(); Size::COUNT]; Color::COUNT],
        }
    }

    pub const fn copies_per_piece() -> u8 {
        Player::COUNT as u8 + 1
    }

    pub const fn count(&self, color: Color, size: Size) -> u8 {
        self.counts[color.index()][size.index()]
    }

    pub fn draw(&mut self, color: Color, size: Size) -> Result<Piece, BankError> {
        let count = &mut self.counts[color.index()][size.index()];
        if *count == 0 {
            return Err(BankError::PieceUnavailable);
        }

        *count -= 1;
        Ok(Piece::new(color, size))
    }

    pub fn return_piece(&mut self, piece: Piece) -> Result<(), BankError> {
        if piece.is_owned() {
            return Err(BankError::OwnedPiece);
        }

        self.counts[piece.color().index()][piece.size().index()] += 1;
        Ok(())
    }
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn bank_starts_with_player_count_plus_one_copies_of_every_piece() {
        let bank = Bank::new();

        assert_eq!(Bank::copies_per_piece(), 3);

        for color in Color::ALL {
            for size in Size::ALL {
                assert_eq!(bank.count(color, size), Bank::copies_per_piece());
            }
        }
    }

    #[test]
    fn drawing_a_piece_returns_it_unowned_and_reduces_its_count() {
        let mut bank = Bank::new();

        let piece = bank.draw(Color::Yellow, Size::Small).expect("piece exists");

        assert_eq!(piece, Piece::new(Color::Yellow, Size::Small));
        assert_eq!(
            bank.count(Color::Yellow, Size::Small),
            Bank::copies_per_piece() - 1
        );
    }

    #[test]
    fn drawing_more_than_available_returns_an_error() {
        let mut bank = Bank::new();

        for _ in 0..Bank::copies_per_piece() {
            bank.draw(Color::Red, Size::Large).expect("piece exists");
        }

        assert_eq!(
            bank.draw(Color::Red, Size::Large),
            Err(BankError::PieceUnavailable)
        );
        assert_eq!(bank.count(Color::Red, Size::Large), 0);
    }

    #[test]
    fn returning_an_unowned_piece_increases_its_count() {
        let mut bank = Bank::new();
        let piece = bank.draw(Color::Green, Size::Medium).expect("piece exists");

        bank.return_piece(piece).expect("unowned piece can return");

        assert_eq!(
            bank.count(Color::Green, Size::Medium),
            Bank::copies_per_piece()
        );
    }

    #[test]
    fn returning_an_owned_piece_is_rejected_without_changing_counts() {
        let mut bank = Bank::new();
        let piece = Piece::owned(Color::Blue, Size::Large, Player::One);

        assert_eq!(bank.return_piece(piece), Err(BankError::OwnedPiece));
        assert_eq!(
            bank.count(Color::Blue, Size::Large),
            Bank::copies_per_piece()
        );
    }
}
