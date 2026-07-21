use serde::{Deserialize, Serialize};

/// Release regions, closed to the vocabulary the database actually uses;
/// unknown region text is a data error to fix, not a value to store.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Region {
    Japan,
    Usa,
    Europe,
    World,
    Taiwan,
    Germany,
    France,
    China,
    Spain,
    Italy,
    Australia,
    UnitedKingdom,
    Korea,
    HongKong,
    Sweden,
    Netherlands,
    Canada,
    Brazil,
}
