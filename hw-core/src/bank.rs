use crate::{Color, Piece, Player, Size};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BankError {
    PieceUnavailable,
    OwnedPiece,
    TooManyPieces {
        color: Color,
        size: Size,
        count: u8,
        max: u8,
    },
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

    pub fn from_counts(counts: [[u8; Size::COUNT]; Color::COUNT]) -> Result<Self, BankError> {
        for color in Color::ALL {
            for size in Size::ALL {
                let count = counts[color.index()][size.index()];
                let max = Self::copies_per_piece();
                if count > max {
                    return Err(BankError::TooManyPieces {
                        color,
                        size,
                        count,
                        max,
                    });
                }
            }
        }

        Ok(Self { counts })
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
    fn bank_can_be_constructed_from_counts() {
        let mut counts = [[Bank::copies_per_piece(); Size::COUNT]; Color::COUNT];
        counts[Color::Red.index()][Size::Small.index()] = 1;

        let bank = Bank::from_counts(counts).expect("counts are valid");

        assert_eq!(bank.count(Color::Red, Size::Small), 1);
        assert_eq!(
            bank.count(Color::Blue, Size::Large),
            Bank::copies_per_piece()
        );
    }

    #[test]
    fn bank_rejects_counts_above_the_copy_limit() {
        let mut counts = [[Bank::copies_per_piece(); Size::COUNT]; Color::COUNT];
        counts[Color::Yellow.index()][Size::Medium.index()] = Bank::copies_per_piece() + 1;

        assert_eq!(
            Bank::from_counts(counts),
            Err(BankError::TooManyPieces {
                color: Color::Yellow,
                size: Size::Medium,
                count: Bank::copies_per_piece() + 1,
                max: Bank::copies_per_piece(),
            })
        );
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
