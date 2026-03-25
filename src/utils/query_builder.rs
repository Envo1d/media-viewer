pub fn build_search_query(input: &str) -> String {
    input
        .split_whitespace()
        .map(|w| format!("{}*", w))
        .collect::<Vec<_>>()
        .join(" AND ")
}
