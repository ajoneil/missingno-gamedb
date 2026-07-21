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
        } else if c != '\'' && c != '\u{2019}' {
            // Apostrophes vanish (links-awakening, not link-s-awakening).
            gap = true;
        }
    }
    slug
}

/// Undo the catalogue article-postfix convention, per subtitle segment:
/// "Legend of Zelda, The - Minish Cap, The" → "The Legend of Zelda - The Minish Cap".
pub fn fix_leading_articles(title: &str) -> String {
    const ARTICLES: [&str; 16] = [
        "The", "A", "An", "Le", "La", "Les", "L'", "Der", "Die", "Das", "El", "Los", "Las", "Il",
        "Lo", "Gli",
    ];
    title
        .split(" - ")
        .map(|segment| {
            for article in ARTICLES {
                if let Some(base) = segment.strip_suffix(&format!(", {article}")) {
                    return if article.ends_with('\'') {
                        format!("{article}{base}")
                    } else {
                        format!("{article} {base}")
                    };
                }
            }
            segment.to_owned()
        })
        .collect::<Vec<_>>()
        .join(" - ")
}

/// Collation form for near-miss detection (shared with every consumer).
pub fn normalize_title(title: &str) -> String {
    missingno_gamedb::normalized_title(title)
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
        assert_eq!(
            slugify("The Legend of Zelda - Link's Awakening DX"),
            "the-legend-of-zelda-links-awakening-dx"
        );
    }

    #[test]
    fn articles_move_to_the_front() {
        assert_eq!(
            fix_leading_articles("Legend of Zelda, The - Minish Cap, The"),
            "The Legend of Zelda - The Minish Cap"
        );
        assert_eq!(fix_leading_articles("Aventure, L'"), "L'Aventure");
        assert_eq!(fix_leading_articles("Pitfall II"), "Pitfall II");
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
