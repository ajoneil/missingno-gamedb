use missingno_gamedb::Region;

/// Directory-name form of a title: lowercase alphanumeric runs joined by `-`.
pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut gap = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(c.to_ascii_lowercase());
            gap = false;
        } else {
            gap = true;
        }
    }
    slug
}

/// Collation form for near-miss detection: casefolded, parentheticals and
/// punctuation stripped.
pub fn normalize_title(title: &str) -> String {
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

/// One No-Intro region word → schema region. `Unknown` maps to no region.
pub fn parse_region(word: &str) -> Option<Option<Region>> {
    let region = match word {
        "Japan" => Region::Japan,
        "USA" => Region::Usa,
        "Europe" => Region::Europe,
        "World" => Region::World,
        "Taiwan" => Region::Taiwan,
        "Germany" => Region::Germany,
        "France" => Region::France,
        "China" => Region::China,
        "Spain" => Region::Spain,
        "Italy" => Region::Italy,
        "Australia" => Region::Australia,
        "United Kingdom" => Region::UnitedKingdom,
        "Korea" => Region::Korea,
        "Hong Kong" => Region::HongKong,
        "Sweden" => Region::Sweden,
        "Netherlands" => Region::Netherlands,
        "Canada" => Region::Canada,
        "Brazil" => Region::Brazil,
        "Unknown" => return Some(None),
        _ => return None,
    };
    Some(Some(region))
}

/// Split a legacy comma-separated region string. Unmappable words come back
/// in the error variant for the report.
pub fn parse_region_list(text: &str) -> Result<Vec<Region>, Vec<String>> {
    let mut regions = Vec::new();
    let mut unknown = Vec::new();
    for word in text.split(',').map(str::trim).filter(|w| !w.is_empty()) {
        match parse_region(word) {
            Some(Some(region)) => regions.push(region),
            Some(None) => {}
            None => unknown.push(word.to_owned()),
        }
    }
    if unknown.is_empty() {
        Ok(regions)
    } else {
        Err(unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_matches_corpus_style() {
        assert_eq!(
            slugify("Pitfall II - Lost Caverns"),
            "pitfall-ii-lost-caverns"
        );
        assert_eq!(slugify("1942"), "1942");
        assert_eq!(slugify("Q*bert"), "q-bert");
    }

    #[test]
    fn normalize_ignores_parentheticals() {
        assert_eq!(normalize_title("15 (WIP)"), normalize_title("15"));
        assert_ne!(normalize_title("15"), normalize_title("16"));
    }

    #[test]
    fn region_lists() {
        assert_eq!(
            parse_region_list("USA, Europe"),
            Ok(vec![Region::Usa, Region::Europe])
        );
        assert_eq!(parse_region_list("Unknown"), Ok(vec![]));
        assert_eq!(
            parse_region_list("Top Secret"),
            Err(vec!["Top Secret".to_owned()])
        );
    }
}
