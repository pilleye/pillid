pub(crate) fn u64_to_string(mut value: u64, alphabet: &[char]) -> String {
    let mut result = String::new();

    while value > 0 {
        let remainder = value % alphabet.len() as u64;
        result.push(alphabet[remainder as usize]);
        value /= alphabet.len() as u64;
    }

    result.chars().rev().collect()
}
