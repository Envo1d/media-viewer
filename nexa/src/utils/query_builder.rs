pub fn build_search_query(input: &str) -> String {
    input
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| {
            let safe = w.replace('"', "\"\"");
            format!("\"{}\"*", safe)
        })
        .collect::<Vec<_>>()
        .join(" ")
}
