/// Core words/tokens
#[derive(
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
    documented::Documented,
    documented::DocumentedFields,
    strum::Display,
    strum::EnumIter,
    strum::EnumString,
)]
#[cfg_attr(test, derive(chaos_theory::Arbitrary))]
#[documented_fields(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Core {
    /// Pop an item off the stack, ignoring it.
    Drop,
    /// Swap the top two elements of the stack.
    Swap,
    /// Duplicate the first element of the stack.
    Rot,
    /// Rotate the top three elements of the stack.
    Dup,
    /// Add the first two elements of the stack.
    Add,
    /// Subtract the second from the first element of the stack.
    Sub,
    /// Multiply the first two elements of the stack.
    Mul,
    /// Divide the second into the first element of the stack.
    Div,
    /// Take the remainder of the second in the first element of the stack.
    Mod,
    /// Pop 3 elements. If the first is zero, push the second back on; otherwise, push the third.
    #[documented_fields(rename = "zero?")]
    #[strum(serialize = "zero?")]
    Zero,
    /// Pop an element off the stack and print it.
    Print,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use chaos_theory::check;

    #[test]
    fn roundtrip() {
        check(|src| {
            let c = src.any::<Core>("core");
            let s = c.to_string();
            let c2 = s.parse::<Core>();
            assert!(c2.is_ok());
            assert_eq!(c2.expect("is_ok"), c);
        });
    }
}
