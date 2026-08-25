use serde::{Deserialize, Serialize};

/// Release regions. Free text drifts across spellings, so regions are a
/// typed vocabulary — a region a catalogue attests earns a variant here.
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
    Argentina,
    Singapore,
    Thailand,
    NewZealand,
}
