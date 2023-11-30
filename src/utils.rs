pub(crate) fn string_size(
    size: usize,
    prefix: &Option<String>,
    timestamp: &Option<String>,
) -> usize {
    let mut size = size;

    if let Some(prefix) = prefix {
        size += prefix.len() + 1;
    }

    if let Some(timestamp) = timestamp {
        size += timestamp.len();
    }

    size
}
