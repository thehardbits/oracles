use crate::{
    traits::{MsgDecode, TimestampDecode},
    Error, Result,
};
use chrono::{DateTime, Utc};
use helium_crypto::PublicKeyBinary;
use helium_proto::HotspotThresholdReportV1;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HotspotThresholdReport {
    pub hotspot_pubkey: PublicKeyBinary,
    pub bytes_threshold: u64,
    pub subscriber_threshold: u32,
    pub timestamp: DateTime<Utc>,
}

impl MsgDecode for HotspotThresholdReport {
    type Msg = HotspotThresholdReportV1;
}

impl TryFrom<HotspotThresholdReportV1> for HotspotThresholdReport {
    type Error = Error;
    fn try_from(v: HotspotThresholdReportV1) -> Result<Self> {
        Ok(Self {
            hotspot_pubkey: v.hotspot_pubkey.into(),
            bytes_threshold: v.bytes_threshold,
            subscriber_threshold: v.subscriber_threshold,
            timestamp: v.timestamp.to_timestamp()?,
        })
    }
}
