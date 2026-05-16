use std::borrow::Cow;

#[inline]
pub fn truncate(s: &str, max_ch: usize) -> Cow<'_, str> {
    if s.chars().count() <= max_ch {
        return Cow::Borrowed(s);
    }
    let body: String = s.chars().take(max_ch.saturating_sub(1)).collect();
    Cow::Owned(format!("{body}…"))
}
