#[derive(Clone, Copy)]
pub struct DivCeilBy {
    divisor: u64,
    magic: u64,
}

impl DivCeilBy {
    pub fn new(divisor: usize) -> Self {
        debug_assert!(divisor >= 2, "reciprocal needs divisor >= 2");
        let divisor = divisor as u64;
        Self {
            divisor,
            magic: ((1u128 << 64) / u128::from(divisor)) as u64 + 1,
        }
    }

    pub fn apply(self, value: usize) -> usize {
        let numerator = value as u64 + (self.divisor - 1);
        debug_assert!(
            numerator <= u64::from(u32::MAX),
            "reciprocal input out of range"
        );
        ((u128::from(numerator) * u128::from(self.magic)) >> 64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_div_ceil_reciprocal_matches_div_ceil() {
        // Every plausible terminal content width against every small message
        // width, plus the boundaries the reciprocal derivation depends on.
        for divisor in 2usize..=512 {
            let div = DivCeilBy::new(divisor);
            for value in 0usize..2048 {
                assert_eq!(
                    div.apply(value),
                    value.div_ceil(divisor),
                    "divisor {divisor}, value {value}"
                );
            }

            for value in [
                divisor - 1,
                divisor,
                divisor + 1,
                u16::MAX as usize,
                u32::MAX as usize - divisor + 1,
            ] {
                assert_eq!(
                    div.apply(value),
                    value.div_ceil(divisor),
                    "divisor {divisor}, value {value}"
                );
            }
        }
    }
}
