use crate::{metrics::Metrics, Settings};
use anyhow::{anyhow, bail, Error, Result};
use chrono::{DateTime, Duration, TimeZone, Utc};
use file_store::file_sink;
use futures::{future::LocalBoxFuture, TryFutureExt};
use helium_anchor_gen::{
    anchor_lang::AccountDeserialize,
    price_oracle::{calculate_current_price, PriceOracleV0},
};
use helium_proto::{BlockchainTokenTypeV1, PriceReportV1};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey as SolPubkey;
use std::{cmp::Ordering, path::PathBuf, str::FromStr};
use task_manager::ManagedTask;
use tokio::{fs, time};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    timestamp: DateTime<Utc>,
    price: u64,
    token_type: BlockchainTokenTypeV1,
}

impl Price {
    fn new(timestamp: DateTime<Utc>, price: u64, token_type: BlockchainTokenTypeV1) -> Self {
        Self {
            timestamp,
            price,
            token_type,
        }
    }
}

pub struct PriceGenerator {
    token_type: BlockchainTokenTypeV1,
    client: RpcClient,
    interval_duration: std::time::Duration,
    last_price_opt: Option<Price>,
    key: Option<SolPubkey>,
    default_price: Option<u64>,
    stale_price_duration: Duration,
    latest_price_file: PathBuf,
    file_sink: Option<file_sink::FileSinkClient>,
}

impl ManagedTask for PriceGenerator {
    fn start_task(
        self: Box<Self>,
        shutdown: triggered::Listener,
    ) -> LocalBoxFuture<'static, anyhow::Result<()>> {
        Box::pin(self.run(shutdown))
    }
}

impl From<Price> for PriceReportV1 {
    fn from(value: Price) -> Self {
        Self {
            timestamp: value.timestamp.timestamp() as u64,
            price: value.price,
            token_type: value.token_type.into(),
        }
    }
}

impl TryFrom<PriceReportV1> for Price {
    type Error = Error;

    fn try_from(value: PriceReportV1) -> Result<Self, Self::Error> {
        let tt: BlockchainTokenTypeV1 = BlockchainTokenTypeV1::from_i32(value.token_type)
            .ok_or_else(|| anyhow!("unsupported token type: {:?}", value.token_type))?;
        Ok(Self {
            timestamp: Utc
                .timestamp_opt(value.timestamp as i64, 0)
                .single()
                .ok_or_else(|| anyhow!("invalid timestamp"))?,
            price: value.price,
            token_type: tt,
        })
    }
}

impl PriceGenerator {
    pub async fn new(
        settings: &Settings,
        token_type: BlockchainTokenTypeV1,
        file_sink: file_sink::FileSinkClient,
    ) -> Result<Self> {
        let client = RpcClient::new(settings.source.clone());
        Ok(Self {
            last_price_opt: None,
            token_type,
            client,
            key: settings.price_key(token_type)?,
            default_price: settings.default_price(token_type),
            interval_duration: settings.interval().to_std()?,
            stale_price_duration: settings.stale_price_duration(),
            latest_price_file: PathBuf::from_str(&settings.cache)?
                .join(format!("{token_type:?}.latest")),
            file_sink: Some(file_sink),
        })
    }

    pub async fn run(mut self, shutdown: triggered::Listener) -> Result<()> {
        match (self.key, self.default_price, self.file_sink.clone()) {
            (Some(key), _, Some(file_sink)) => self.run_with_key(key, file_sink, &shutdown).await,
            (None, Some(defaut_price), Some(file_sink)) => {
                self.run_with_default(defaut_price, file_sink, &shutdown)
                    .await
            }
            _ => {
                tracing::warn!(
                    "stopping price generator for {:?}, not configured",
                    self.token_type
                );
                Ok(())
            }
        }
    }

    async fn run_with_default(
        &mut self,
        default_price: u64,
        file_sink: file_sink::FileSinkClient,
        shutdown: &triggered::Listener,
    ) -> Result<()> {
        tracing::info!(
            "starting default price generator for {:?}, using price {default_price}",
            self.token_type
        );
        let mut trigger = time::interval(self.interval_duration);

        loop {
            tokio::select! {
                biased;
                _ = shutdown.clone() => break,
                _ = trigger.tick() => {
                    let price = Price::new(Utc::now(), default_price, self.token_type);
                    let price_report = PriceReportV1::from(price);
                    tracing::info!("updating {:?} with default price: {}", self.token_type, default_price);
                    file_sink.write(price_report, []).await?;
                }
            }
        }

        tracing::info!("stopping default price generator for {:?}", self.token_type);
        Ok(())
    }

    async fn run_with_key(
        &mut self,
        key: SolPubkey,
        file_sink: file_sink::FileSinkClient,
        shutdown: &triggered::Listener,
    ) -> Result<()> {
        tracing::info!("starting price generator for {:?}", self.token_type);
        let mut trigger = time::interval(self.interval_duration);
        self.last_price_opt = self.read_price_file().await;

        loop {
            tokio::select! {
                biased;
                _ = shutdown.clone() => break,
                _ = trigger.tick() => self.handle(&key, &file_sink).await?,
            }
        }

        tracing::info!("stopping price generator for {:?}", self.token_type);
        Ok(())
    }

    async fn handle(
        &mut self,
        key: &SolPubkey,
        file_sink: &file_sink::FileSinkClient,
    ) -> Result<()> {
        let price_opt = match if matches!(
            self.token_type,
            BlockchainTokenTypeV1::Hnt | BlockchainTokenTypeV1::Mobile
        ) {
            // HNT and Mobile prices are currently supported by pyth
            self.get_pyth_price(key).await
        } else {
            self.get_standard_price(key).await
        } {
            Ok(new_price) => {
                tracing::info!(
                    "updating price for {:?} to {}",
                    self.token_type,
                    new_price.price
                );
                self.last_price_opt = Some(new_price.clone());
                self.write_price_file(&new_price).await;

                Metrics::update(
                    "price_update_counter".to_string(),
                    self.token_type,
                    new_price.price as f64,
                );

                Some(new_price)
            }
            Err(err) => {
                tracing::error!(
                    "error in retrieving new price for {:?}: {err:?}",
                    self.token_type
                );

                match &self.last_price_opt {
                    Some(old_price) if self.is_valid(old_price) => {
                        Metrics::update(
                            "price_stale_counter".to_string(),
                            self.token_type,
                            old_price.price as f64,
                        );

                        Some(Price::new(
                            Utc::now(),
                            old_price.price,
                            old_price.token_type,
                        ))
                    }
                    Some(_old_price) => {
                        tracing::warn!(
                            "stale price for {:?} is too old, discarding",
                            self.token_type
                        );
                        self.last_price_opt = None;
                        None
                    }
                    None => None,
                }
            }
        };

        if let Some(price) = price_opt {
            let price_report = PriceReportV1::from(price);
            tracing::debug!("price_report: {:?}", price_report);
            file_sink.write(price_report, []).await?;
        }

        Ok(())
    }

    async fn get_pyth_price(&self, price_key: &SolPubkey) -> Result<Price> {
        let mut account = self.client.get_account(price_key).await?;
        let price_oracle = pyth_sdk_solana::load_price_feed_from_account(price_key, &mut account)?;
        let curr_price = price_oracle
            .get_ema_price_no_older_than(Utc::now().timestamp(), self.interval_duration.as_secs())
            .ok_or_else(|| anyhow!("No new price in the given interval"))?;

        if curr_price.price < 0 {
            bail!("Price is less than zero");
        }

        // Remove the confidence interval from the price to get the most optimistic price:
        let optimistic_price = curr_price.price as u64 + curr_price.conf * 2;

        // We want the price to have a resulting exponent of 10^-6
        // I don't think it's possible for pyth to give us anything other than -8, but we make
        // this robust just in case:
        let exp = curr_price.expo + 6;
        let adjusted_optimistic_price = match exp.cmp(&0) {
            Ordering::Less => optimistic_price / 10_u64.pow(exp.unsigned_abs()),
            Ordering::Greater => optimistic_price * 10_u64.pow(exp as u32),
            _ => optimistic_price,
        };

        Ok(Price::new(
            DateTime::from_timestamp(curr_price.publish_time, 0).ok_or_else(|| {
                anyhow!(
                    "Invalid publish time for price: {}",
                    curr_price.publish_time
                )
            })?,
            adjusted_optimistic_price,
            self.token_type,
        ))
    }

    async fn get_standard_price(&self, price_key: &SolPubkey) -> Result<Price> {
        let price_oracle_v0_data = self.client.get_account_data(price_key).await?;
        let mut price_oracle_v0_data = price_oracle_v0_data.as_ref();
        let price_oracle_v0 = PriceOracleV0::try_deserialize(&mut price_oracle_v0_data)?;

        let current_time = Utc::now();
        let current_timestamp = current_time.timestamp();

        calculate_current_price(&price_oracle_v0.oracles, current_timestamp)
            .map(|price| {
                tracing::debug!(
                    "got price: {:?} for token_type: {:?}",
                    price,
                    self.token_type
                );
                Price::new(current_time, price, self.token_type)
            })
            .ok_or_else(|| anyhow!("unable to fetch price!"))
    }

    fn is_valid(&self, price: &Price) -> bool {
        price.timestamp > Utc::now() - self.stale_price_duration
    }

    async fn read_price_file(&self) -> Option<Price> {
        fs::read_to_string(&self.latest_price_file)
            .map_err(|err| format!("{err:?}"))
            .and_then(|contents| async move {
                serde_json::from_str::<Price>(&contents).map_err(|err| format!("{err:?}"))
            })
            .await
            .map_err(|err| {
                tracing::warn!(token = ?self.token_type, "unable to read latest price file due to {err}");
                err
            })
            .ok()
    }

    async fn write_price_file(&self, price: &Price) {
        let result = async { serde_json::to_string_pretty(price) }
            .map_err(|err| format!("{err:?}"))
            .and_then(|json| {
                fs::write(&self.latest_price_file, json).map_err(|err| format!("{err:?}"))
            })
            .await;

        match result {
            Ok(_) => (),
            Err(err) => {
                tracing::warn!(token = ?self.token_type, "unable to save latest price file due to {err}");
            }
        }
    }
}
