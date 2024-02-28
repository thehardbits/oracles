use crate::{
    error::DecodeError,
    traits::{MsgDecode, MsgTimestamp, TimestampDecode, TimestampEncode},
    Error, Result,
};
use chrono::{DateTime, Utc};
use helium_crypto::PublicKeyBinary;
use helium_proto::services::poc_mobile::{
    HotspotThresholdIngestReportV1, HotspotThresholdReportReqV1,
    HotspotThresholdReportVerificationStatus, VerifiedHotspotThresholdIngestReportV1,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HotspotThresholdReportReq {
    pub hotspot_pubkey: PublicKeyBinary,
    pub bytes_threshold: u64,
    pub subscriber_threshold: u32,
    pub threshold_timestamp: DateTime<Utc>,
    pub carrier_pub_key: PublicKeyBinary,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerifiedHotspotThresholdIngestReport {
    pub report: HotspotThresholdIngestReport,
    pub status: HotspotThresholdReportVerificationStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HotspotThresholdIngestReport {
    pub received_timestamp: DateTime<Utc>,
    pub report: HotspotThresholdReportReq,
}

impl MsgDecode for HotspotThresholdReportReq {
    type Msg = HotspotThresholdReportReqV1;
}

impl MsgDecode for HotspotThresholdIngestReport {
    type Msg = HotspotThresholdIngestReportV1;
}

impl MsgDecode for VerifiedHotspotThresholdIngestReport {
    type Msg = VerifiedHotspotThresholdIngestReportV1;
}

impl TryFrom<HotspotThresholdReportReqV1> for HotspotThresholdReportReq {
    type Error = Error;
    fn try_from(v: HotspotThresholdReportReqV1) -> Result<Self> {
        Ok(Self {
            hotspot_pubkey: v.hotspot_pubkey.into(),
            bytes_threshold: v.bytes_threshold,
            subscriber_threshold: v.subscriber_threshold,
            threshold_timestamp: v.threshold_timestamp.to_timestamp()?,
            carrier_pub_key: v.carrier_pub_key.into(),
        })
    }
}

impl From<HotspotThresholdReportReq> for HotspotThresholdReportReqV1 {
    fn from(v: HotspotThresholdReportReq) -> Self {
        let threshold_timestamp = v.threshold_timestamp.timestamp() as u64;
        Self {
            hotspot_pubkey: v.hotspot_pubkey.into(),
            bytes_threshold: v.bytes_threshold,
            subscriber_threshold: v.subscriber_threshold,
            threshold_timestamp,
            carrier_pub_key: v.carrier_pub_key.into(),
            signature: vec![],
        }
    }
}

impl MsgTimestamp<Result<DateTime<Utc>>> for HotspotThresholdReportReqV1 {
    fn timestamp(&self) -> Result<DateTime<Utc>> {
        self.threshold_timestamp.to_timestamp()
    }
}

impl MsgTimestamp<u64> for HotspotThresholdReportReq {
    fn timestamp(&self) -> u64 {
        self.threshold_timestamp.encode_timestamp()
    }
}

impl MsgTimestamp<Result<DateTime<Utc>>> for HotspotThresholdIngestReportV1 {
    fn timestamp(&self) -> Result<DateTime<Utc>> {
        self.received_timestamp.to_timestamp_millis()
    }
}

impl MsgTimestamp<u64> for HotspotThresholdIngestReport {
    fn timestamp(&self) -> u64 {
        self.received_timestamp.encode_timestamp_millis()
    }
}

impl MsgTimestamp<Result<DateTime<Utc>>> for VerifiedHotspotThresholdIngestReportV1 {
    fn timestamp(&self) -> Result<DateTime<Utc>> {
        self.timestamp.to_timestamp_millis()
    }
}

impl MsgTimestamp<u64> for VerifiedHotspotThresholdIngestReport {
    fn timestamp(&self) -> u64 {
        self.timestamp.encode_timestamp_millis()
    }
}

impl TryFrom<HotspotThresholdIngestReportV1> for HotspotThresholdIngestReport {
    type Error = Error;
    fn try_from(v: HotspotThresholdIngestReportV1) -> Result<Self> {
        Ok(Self {
            received_timestamp: v.timestamp()?,
            report: v
                .report
                .ok_or_else(|| Error::not_found("ingest hotspot threshold ingest report"))?
                .try_into()?,
        })
    }
}

impl From<HotspotThresholdIngestReport> for HotspotThresholdIngestReportV1 {
    fn from(v: HotspotThresholdIngestReport) -> Self {
        let received_timestamp = v.timestamp();
        let report: HotspotThresholdReportReqV1 = v.report.into();
        Self {
            received_timestamp,
            report: Some(report),
        }
    }
}

impl TryFrom<VerifiedHotspotThresholdIngestReportV1> for VerifiedHotspotThresholdIngestReport {
    type Error = Error;
    fn try_from(v: VerifiedHotspotThresholdIngestReportV1) -> Result<Self> {
        let status =
            HotspotThresholdReportVerificationStatus::from_i32(v.status).ok_or_else(|| {
                DecodeError::unsupported_status_reason(
                    "verified_hotspot_threshold_ingest_report_v1",
                    v.status,
                )
            })?;
        Ok(Self {
            report: v
                .report
                .ok_or_else(|| Error::not_found("ingest hotspot threshold ingest report"))?
                .try_into()?,
            status,
            timestamp: v.timestamp.to_timestamp()?,
        })
    }
}

impl From<VerifiedHotspotThresholdIngestReport> for VerifiedHotspotThresholdIngestReportV1 {
    fn from(v: VerifiedHotspotThresholdIngestReport) -> Self {
        let timestamp = v.timestamp();
        let report: HotspotThresholdIngestReportV1 = v.report.into();
        Self {
            report: Some(report),
            status: v.status as i32,
            timestamp,
        }
    }
}
