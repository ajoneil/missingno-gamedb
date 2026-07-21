use serde::{Deserialize, Serialize};

/// Where a release can legitimately be obtained. A release carries a list,
/// preference-ordered; an empty list means dump-only (verify your own copy).
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum Source {
    Download {
        url: String,
    },
    HomebrewHub {
        slug: String,
        filename: String,
    },
    Itch {
        url: String,
        /// None until someone checks the store page.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        paid: Option<bool>,
    },
    SteamBundled {
        app_id: u32,
        path: String,
    },
}
