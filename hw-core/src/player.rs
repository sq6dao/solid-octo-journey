#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Player {
    One,
    Two,
}

impl Player {
    pub const COUNT: usize = 2;
    pub const ALL: [Self; Self::COUNT] = [Self::One, Self::Two];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn players_are_the_two_supported_seats() {
        assert_eq!(Player::ALL, [Player::One, Player::Two]);
    }
}
