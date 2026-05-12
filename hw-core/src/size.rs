#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    pub const COUNT: usize = 3;
    pub const ALL: [Self; Self::COUNT] = [Self::Small, Self::Medium, Self::Large];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Small => 0,
            Self::Medium => 1,
            Self::Large => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_the_three_piece_sizes() {
        assert_eq!(Size::ALL, [Size::Small, Size::Medium, Size::Large]);
    }

    #[test]
    fn sizes_are_ordered_by_ship_strength() {
        assert!(Size::Small < Size::Medium);
        assert!(Size::Medium < Size::Large);
        assert!(Size::Large >= Size::Small);
    }
}
