/// Collation form for duplicate/near-miss detection: casefolded, with
/// parentheticals and punctuation stripped.
pub fn normalized_title(title: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in title.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            c if depth == 0 && c.is_ascii_alphanumeric() => out.push(c.to_ascii_lowercase()),
            _ => {}
        }
    }
    out
}
