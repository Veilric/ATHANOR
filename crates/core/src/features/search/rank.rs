pub fn rank(name: &str, query: &str) -> Option<(u8, usize)> {
    if name == query {
        return Some((0, 0));
    }
    if name.starts_with(query) {
        return Some((1, name.len() - query.len()));
    }
    if let Some(at) = name.find(query) {
        let word = at == 0
            || name[..at]
                .chars()
                .next_back()
                .is_some_and(|c| !c.is_alphanumeric());
        return Some((if word { 2 } else { 3 }, at));
    }
    let mut wanted = query.chars();
    let mut next = wanted.next()?;
    let mut first = None;
    for (at, ch) in name.chars().enumerate() {
        if ch == next {
            first.get_or_insert(at);
            match wanted.next() {
                Some(ch) => next = ch,
                None => return Some((4, at - first.unwrap_or(at))),
            }
        }
    }
    None
}
