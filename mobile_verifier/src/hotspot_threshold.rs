use chrono::{DateTime, Utc};
use file_store::file_info_poller::FileInfoStream;
use file_store::{
    file_sink::FileSinkClient,
    mobile_hotspot_threshold::{
        HotspotThresholdIngestReport, HotspotThresholdReportReq,
        VerifiedHotspotThresholdIngestReport,
    },
};
use futures::{StreamExt, TryStreamExt};
use futures_util::TryFutureExt;
use helium_crypto::PublicKeyBinary;
use helium_proto::services::poc_mobile::VerifiedHotspotThresholdIngestReportV1;
use helium_proto::services::{
    mobile_config::NetworkKeyRole, poc_mobile::HotspotThresholdReportVerificationStatus,
};
use mobile_config::client::authorization_client::AuthorizationVerifier;
use sqlx::{postgres::PgRow, FromRow, PgPool, Postgres, Row, Transaction};
use std::{collections::HashMap, ops::Range};
use task_manager::ManagedTask;
use tokio::sync::mpsc::Receiver;

pub struct HotspotThresholdIngestor<AV> {
    pool: PgPool,
    reports_receiver: Receiver<FileInfoStream<HotspotThresholdIngestReport>>,
    verified_report_sink: FileSinkClient,
    authorization_verifier: AV,
}

impl<AV> ManagedTask for HotspotThresholdIngestor<AV>
where
    AV: AuthorizationVerifier + Send + Sync + 'static,
{
    fn start_task(
        self: Box<Self>,
        shutdown: triggered::Listener,
    ) -> futures_util::future::LocalBoxFuture<'static, anyhow::Result<()>> {
        let handle = tokio::spawn(self.run(shutdown));
        Box::pin(
            handle
                .map_err(anyhow::Error::from)
                .and_then(|result| async move { result.map_err(anyhow::Error::from) }),
        )
    }
}

impl<AV> HotspotThresholdIngestor<AV>
where
    AV: AuthorizationVerifier,
{
    pub fn new(
        pool: sqlx::Pool<Postgres>,
        reports_receiver: Receiver<FileInfoStream<HotspotThresholdIngestReport>>,
        verified_report_sink: FileSinkClient,
        authorization_verifier: AV,
    ) -> Self {
        Self {
            pool,
            reports_receiver,
            verified_report_sink,
            authorization_verifier,
        }
    }

    async fn run(mut self, shutdown: triggered::Listener) -> anyhow::Result<()> {
        tracing::info!("starting HotspotThresholdIngestor");
        loop {
            tokio::select! {
                biased;
                _ = shutdown.clone() => break,
                Some(file) = self.reports_receiver.recv() => {
                    self.process_file(file).await?;
                }
            }
        }
        tracing::info!("stopping HotspotThresholdIngestor");
        Ok(())
    }

    async fn process_file(
        &self,
        file_info_stream: FileInfoStream<HotspotThresholdIngestReport>,
    ) -> anyhow::Result<()> {
        let mut transaction = self.pool.begin().await?;
        file_info_stream
            .into_stream(&mut transaction)
            .await?
            .map(anyhow::Ok)
            .try_fold(transaction, |mut transaction, ingest_report| async move {
                // verifiy the report
                let verified_report_status = self.verify_report(&ingest_report.report).await;

                // if the report is valid then save to the db
                // and thus available to the rewarder
                if verified_report_status
                    == HotspotThresholdReportVerificationStatus::ThresholdReportStatusValid
                {
                    save(&ingest_report, &mut transaction).await?;
                }

                // write out paper trail of verified report, valid or invalid
                let verified_report_proto: VerifiedHotspotThresholdIngestReportV1 =
                    VerifiedHotspotThresholdIngestReport {
                        report: ingest_report,
                        status: verified_report_status,
                        timestamp: Utc::now(),
                    }
                    .into();
                self.verified_report_sink
                    .write(
                        verified_report_proto,
                        &[("report_status", verified_report_status.as_str_name())],
                    )
                    .await?;
                Ok(transaction)
            })
            .await?
            .commit()
            .await?;
        Ok(())
    }

    async fn verify_report(
        &self,
        report: &HotspotThresholdReportReq,
    ) -> HotspotThresholdReportVerificationStatus {
        if !self.verify_known_carrier_key(&report.carrier_pub_key).await {
            return HotspotThresholdReportVerificationStatus::ThresholdReportStatusInvalidCarrierKey;
        };
        HotspotThresholdReportVerificationStatus::ThresholdReportStatusValid
    }

    async fn verify_known_carrier_key(&self, public_key: &PublicKeyBinary) -> bool {
        match self
            .authorization_verifier
            .verify_authorized_key(public_key, NetworkKeyRole::MobileCarrier)
            .await
        {
            Ok(res) => res,
            Err(_err) => false,
        }
    }
}

pub async fn save(
    ingest_report: &HotspotThresholdIngestReport,
    db: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
            INSERT INTO hotspot_threshold (hotspot_pubkey, bytes_threshold, subscriber_threshold, timestamp)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (hotspot_pubkey)
            DO UPDATE SET
            bytes_threshold = EXCLUDED.bytes_threshold,
            subscriber_threshold = EXCLUDED.subscriber_threshold,
            updated_at = now()
            "#,
    )
    .bind(ingest_report.report.hotspot_pubkey.to_string())
    .bind(ingest_report.report.bytes_threshold as i64)
    .bind(ingest_report.report.subscriber_threshold as i32)
    .bind(ingest_report.report.threshold_timestamp)
    .execute(&mut *db)
    .await?;
    Ok(())
}

pub struct HotspotThreshold {
    hotspot_pubkey: PublicKeyBinary,
    bytes_threshold: u64,
    subscriber_threshold: u32,
}

impl FromRow<'_, PgRow> for HotspotThreshold {
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        Ok(Self {
            hotspot_pubkey: row.try_get("hotspot_pubkey")?,
            bytes_threshold: row.try_get::<i64, _>("bytes_threshold")? as u64,
            subscriber_threshold: row.try_get::<i32, _>("subscriber_threshold")? as u32,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct VerifiedHotspotThresholds {
    gateways: HashMap<PublicKeyBinary, (u64, u32)>,
}

impl VerifiedHotspotThresholds {
    pub fn insert(&mut self, key: PublicKeyBinary, value: (u64, u32)) {
        self.gateways.insert(key, value);
    }

    pub fn is_verified(&self, key: &PublicKeyBinary) -> bool {
        self.gateways.contains_key(key)
    }
}

pub async fn verified_hotspot_thresholds(
    pool: &sqlx::Pool<Postgres>,
    reward_period: &Range<DateTime<Utc>>,
) -> Result<VerifiedHotspotThresholds, sqlx::Error> {
    let mut rows = sqlx::query_as::<_, HotspotThreshold>(
        "select hotspot_pubkey, bytes_threshold, subscriber_threshold from hotspot_threshold where timestamp >= $1",
    )
    .bind(reward_period.start)
    .fetch(pool);
    let mut map = VerifiedHotspotThresholds::default();
    while let Some(row) = rows.try_next().await? {
        map.insert(
            row.hotspot_pubkey,
            (row.bytes_threshold, row.subscriber_threshold),
        );
    }
    Ok(map)
}
