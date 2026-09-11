pub const TIME_LENGTH: u16 = 8;
pub const NICKNAME_LENGTH: u16 = 10;
pub const SEPARATOR_LENGTH: u16 = 2;
pub const META_LENGTH: u16 = NICKNAME_LENGTH
    .saturating_add(TIME_LENGTH)
    .saturating_add(SEPARATOR_LENGTH);
