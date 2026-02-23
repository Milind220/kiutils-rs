#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KiCadSeries {
    V9,
    V10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VersionPolicy {
    pub target: KiCadSeries,
    pub reject_older: bool,
}

impl Default for VersionPolicy {
    fn default() -> Self {
        Self {
            target: KiCadSeries::V10,
            reject_older: true,
        }
    }
}

impl VersionPolicy {
    pub const V9_MIN: i32 = 20240101;
    pub const V10_BASE: i32 = 20260101;

    pub fn accepts(&self, version: i32) -> bool {
        version >= Self::V9_MIN
    }

    pub fn is_future_for_target(&self, version: i32) -> bool {
        match self.target {
            KiCadSeries::V9 => version > Self::V9_MIN,
            KiCadSeries::V10 => version > Self::V10_BASE,
        }
    }
}
