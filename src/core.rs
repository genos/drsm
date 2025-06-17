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
#[strum(serialize_all = "lowercase")]
pub enum Core {
    /// Pop an item off the stack, ignoring it.
    Drop,
    /// Swap the top two elements of the stack.
    Swap,
    /// Duplicate the first element of the stack.
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
    #[strum(serialize = "zero?")]
    Zero,
    /// Pop an element off the stack and print it.
    Print,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip(c in core()) {
            let s = c.to_string();
            let c2 = s.parse::<Core>();
            prop_assert!(c2.is_ok());
            prop_assert_eq!(c2.unwrap(), c);
        }
    }

    pub fn core() -> impl Strategy<Value = Core> {
        prop_oneof![
            Just(Core::Drop),
            Just(Core::Swap),
            Just(Core::Dup),
            Just(Core::Add),
            Just(Core::Sub),
            Just(Core::Mul),
            Just(Core::Div),
            Just(Core::Mod),
            Just(Core::Zero),
            Just(Core::Print),
        ]
    }
}
