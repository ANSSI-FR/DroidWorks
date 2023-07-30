#[must_use]
pub(crate) fn hexlify(arr: &[u8]) -> String {
    arr.iter()
        .map(|x| format!("{x:02x}"))
        .collect::<Vec<String>>()
        .concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexlify_test() {
        assert_eq!(hexlify(&vec![15, 60, 99]), String::from("0f3c63"));
    }
}
