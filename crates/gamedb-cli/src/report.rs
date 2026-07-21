use std::{fs, io, path::Path};

/// Accumulates everything a migration refuses to decide, rendered as markdown.
#[derive(Default, Debug)]
pub struct Report {
    sections: Vec<(String, Vec<String>)>,
}

impl Report {
    pub fn add(&mut self, section: &str, item: String) {
        match self.sections.iter_mut().find(|(name, _)| name == section) {
            Some((_, items)) => items.push(item),
            None => self.sections.push((section.to_owned(), vec![item])),
        }
    }

    pub fn item_count(&self) -> usize {
        self.sections.iter().map(|(_, items)| items.len()).sum()
    }

    pub fn render(&self, title: &str) -> String {
        let mut out = format!("# {title}\n");
        if self.sections.is_empty() {
            out.push_str("\nNothing to review.\n");
        }
        for (name, items) in &self.sections {
            out.push_str(&format!("\n## {name} ({})\n\n", items.len()));
            for item in items {
                out.push_str(&format!("- {item}\n"));
            }
        }
        out
    }

    pub fn write(&self, path: &Path, title: &str) -> io::Result<()> {
        fs::write(path, self.render(title))
    }
}
