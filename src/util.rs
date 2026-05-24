/// Greatest-common-divisor via Euclidean algorithm.
/// Returns 0 when both inputs are 0; callers must guard against dividing by the result.
pub fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gcd_basics() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(7, 3), 1);
        assert_eq!(gcd(6, 0), 6);
        assert_eq!(gcd(0, 5), 5);
    }
}
