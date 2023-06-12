use crate::{entropy::Entropy, metrics::Metrics, poc_report::Report, Settings};
use chrono::Duration;
use file_store::{
    file_sink::FileSinkClient,
    iot_beacon_report::IotBeaconIngestReport,
    iot_invalid_poc::IotInvalidBeaconReport,
    iot_invalid_poc::IotInvalidWitnessReport,
    iot_witness_report::IotWitnessIngestReport,
    traits::{IngestId, MsgDecode},
};
use futures::{
    future::LocalBoxFuture,
    stream::{self, StreamExt},
};
use helium_proto::services::poc_lora::{
    InvalidParticipantSide, InvalidReason, LoraInvalidBeaconReportV1, LoraInvalidWitnessReportV1,
};
use lazy_static::lazy_static;
use sqlx::{PgPool, Postgres};
use std::ops::DerefMut;
use task_manager::ManagedTask;
use tokio::{
    sync::Mutex,
    time::{self, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;

const DB_POLL_TIME: time::Duration = time::Duration::from_secs(60 * 35);
const PURGER_WORKERS: usize = 50;

lazy_static! {
    /// the period after which a beacon report in the DB will be deemed stale
    static ref BEACON_STALE_PERIOD: Duration = Duration::minutes(45);
    /// the period after which a witness report in the DB will be deemed stale
    static ref WITNESS_STALE_PERIOD: Duration = Duration::minutes(45);
    /// the period after which an entropy entry in the DB will be deemed stale
    static ref ENTROPY_STALE_PERIOD: Duration = Duration::minutes(60);
}

pub struct Purger {
    pool: PgPool,
    base_stale_period: Duration,
    invalid_beacon_sink: FileSinkClient,
    invalid_witness_sink: FileSinkClient,
}

#[derive(thiserror::Error, Debug)]
#[error("error creating purger: {0}")]
pub struct NewPurgerError(#[from] db_store::Error);

impl ManagedTask for Purger {
    fn start_task(
        self: Box<Self>,
        token: CancellationToken,
    ) -> LocalBoxFuture<'static, anyhow::Result<()>> {
        Box::pin(self.run(token))
    }
}

impl Purger {
    pub async fn from_settings(
        settings: &Settings,
        pool: PgPool,
        invalid_beacon_sink: FileSinkClient,
        invalid_witness_sink: FileSinkClient,
    ) -> Result<Self, NewPurgerError> {
        let base_stale_period = settings.base_stale_period();
        Ok(Self {
            pool,
            base_stale_period,
            invalid_beacon_sink,
            invalid_witness_sink,
        })
    }

    pub async fn run(self, token: CancellationToken) -> anyhow::Result<()> {
        tracing::info!("starting purger");

        let mut db_timer = time::interval(DB_POLL_TIME);
        db_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                _ = db_timer.tick() =>
                    match self.handle_db_tick().await {
                    Ok(()) => (),
                    Err(err) => {
                        tracing::error!("fatal purger error: {err:?}");
                    }
                }
            }
        }
        tracing::info!("stopping purger");
        Ok(())
    }

    async fn handle_db_tick(&self) -> anyhow::Result<()> {
        // pull stale beacons and witnesses
        // for each we have to write out an invalid report to S3
        // as these wont have previously resulted in a file going to s3
        // once the report is safely on s3 we can then proceed to purge from the db
        let beacon_stale_period = self.base_stale_period + *BEACON_STALE_PERIOD;
        tracing::info!(
            "starting query get_stale_pending_beacons with stale period: {beacon_stale_period}"
        );
        let stale_beacons = Report::get_stale_beacons(&self.pool, beacon_stale_period).await?;
        tracing::info!("completed query get_stale_beacons");
        tracing::info!("purging {:?} stale beacons", stale_beacons.len());

        let tx = Mutex::new(self.pool.begin().await?);
        stream::iter(stale_beacons)
            .for_each_concurrent(PURGER_WORKERS, |report| async {
                match self.handle_purged_beacon(&tx, report).await {
                    Ok(()) => (),
                    Err(err) => {
                        tracing::warn!("failed to purge beacon: {err:?}")
                    }
                }
            })
            .await;
        self.invalid_beacon_sink.commit().await?;
        tx.into_inner().commit().await?;

        let witness_stale_period = self.base_stale_period + *WITNESS_STALE_PERIOD;
        tracing::info!(
            "starting query get_stale_pending_witnesses with stale period: {witness_stale_period}"
        );
        let stale_witnesses = Report::get_stale_witnesses(&self.pool, witness_stale_period).await?;
        tracing::info!("completed query get_stale_witnesses");
        let num_stale_witnesses = stale_witnesses.len();
        tracing::info!("purging {num_stale_witnesses} stale witnesses");

        let tx = Mutex::new(self.pool.begin().await?);
        stream::iter(stale_witnesses)
            .for_each_concurrent(PURGER_WORKERS, |report| async {
                match self.handle_purged_witness(&tx, report).await {
                    Ok(()) => (),
                    Err(err) => {
                        tracing::warn!("failed to purge witness: {err:?}")
                    }
                }
            })
            .await;
        self.invalid_witness_sink.commit().await?;
        tx.into_inner().commit().await?;
        tracing::info!("completed purging {num_stale_witnesses} stale witnesses");

        // purge any stale entropy, no need to output anything to s3 here
        _ = Entropy::purge(&self.pool, self.base_stale_period + *ENTROPY_STALE_PERIOD).await;
        Ok(())
    }

    async fn handle_purged_beacon(
        &self,
        tx: &Mutex<sqlx::Transaction<'_, Postgres>>,
        db_beacon: Report,
    ) -> anyhow::Result<()> {
        let beacon_buf: &[u8] = &db_beacon.report_data;
        let beacon_report = IotBeaconIngestReport::decode(beacon_buf)?;
        let beacon_id = beacon_report.ingest_id();
        let beacon = &beacon_report.report;
        let received_timestamp = beacon_report.received_timestamp;
        let invalid_beacon_proto: LoraInvalidBeaconReportV1 = IotInvalidBeaconReport {
            received_timestamp,
            reason: InvalidReason::Stale,
            report: beacon.clone(),
        }
        .into();

        self.invalid_beacon_sink
            .write(
                invalid_beacon_proto,
                &[("reason", InvalidReason::Stale.as_str_name())],
            )
            .await?;
        // delete the report from the DB
        Report::delete_report(tx.lock().await.deref_mut(), &beacon_id).await?;
        Metrics::decrement_num_beacons();
        Ok(())
    }

    async fn handle_purged_witness(
        &self,
        tx: &Mutex<sqlx::Transaction<'_, Postgres>>,
        db_witness: Report,
    ) -> anyhow::Result<()> {
        let witness_buf: &[u8] = &db_witness.report_data;
        let witness_report = IotWitnessIngestReport::decode(witness_buf)?;
        let witness_id = witness_report.ingest_id();
        let received_timestamp = witness_report.received_timestamp;
        let invalid_witness_report_proto: LoraInvalidWitnessReportV1 = IotInvalidWitnessReport {
            received_timestamp,
            report: witness_report.report,
            reason: InvalidReason::Stale,
            participant_side: InvalidParticipantSide::Witness,
        }
        .into();

        self.invalid_witness_sink
            .write(
                invalid_witness_report_proto,
                &[("reason", InvalidReason::Stale.as_str_name())],
            )
            .await?;

        // delete the report from the DB
        Report::delete_report(tx.lock().await.deref_mut(), &witness_id).await?;
        Ok(())
    }
}
