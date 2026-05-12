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

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Red => 0,
            Self::Yellow => 1,
            Self::Green => 2,
            Self::Blue => 3,
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
}
