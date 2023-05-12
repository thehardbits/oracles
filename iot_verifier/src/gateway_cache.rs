use futures::stream::StreamExt;
use helium_crypto::PublicKeyBinary;
use iot_config::{
    client::{Client as IotConfigClient, ClientError as IotConfigClientError},
    gateway_info::{GatewayInfo, GatewayInfoResolver},
};
use rand::{rngs::OsRng, Rng};
use retainer::Cache;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;

/// how long each cached items takes to expire ( 12 hours in seconds)
const CACHE_TTL: Duration = Duration::from_secs(12 * (60 * 60));
/// how often to evict expired items from the cache ( every 1 hour)
const CACHE_EVICTION_FREQUENCY: Duration = Duration::from_secs(60 * 60);
/// as the cache is prewarmed, this results in all entries to the cache
/// being inserted in a short window of time
/// jitter is added to CACHE_TTL per entry to the cache
/// in order to avoid the potential for all items in the cache
/// expiring close together and resulting in a thundering herd of requests
/// to config service
const CACHE_TTL_JITTER_PERCENT: u64 = 10;

pub struct GatewayCache {
    pub iot_config_client: IotConfigClient,
    pub cache: Arc<Cache<PublicKeyBinary, GatewayInfo>>,
    pub cache_monitor: JoinHandle<()>,
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayCacheError {
    #[error("gateway not found: {0}")]
    GatewayNotFound(PublicKeyBinary),
    #[error("error querying iot config service")]
    IotConfigClient(#[from] IotConfigClientError),
}

impl GatewayCache {
    pub fn from_settings(iot_config_client: IotConfigClient) -> Self {
        let cache = Arc::new(Cache::<PublicKeyBinary, GatewayInfo>::new());
        let clone = cache.clone();
        // monitor cache to handle evictions
        let cache_monitor =
            tokio::spawn(async move { clone.monitor(4, 0.25, CACHE_EVICTION_FREQUENCY).await });
        Self {
            iot_config_client,
            cache,
            cache_monitor,
        }
    }

    pub async fn prewarm(&self) -> anyhow::Result<()> {
        tracing::info!("starting prewarming gateway cache");
        let mut gw_stream = self
            .iot_config_client
            .clone()
            .stream_gateways_info()
            .await?;
        while let Some(gateway_info) = gw_stream.next().await {
            _ = self.insert(gateway_info).await;
        }
        tracing::info!("completed prewarming gateway cache");
        Ok(())
    }

    pub async fn resolve_gateway_info(
        &self,
        address: &PublicKeyBinary,
    ) -> Result<GatewayInfo, GatewayCacheError> {
        match self.cache.get(address).await {
            Some(hit) => {
                metrics::increment_counter!("oracles_iot_verifier_gateway_cache_hit");
                Ok(hit.value().clone())
            }
            _ => {
                tracing::debug!("cache miss: {:?}", address);
                metrics::increment_counter!("oracles_iot_verifier_gateway_cache_miss");
                match self
                    .iot_config_client
                    .clone()
                    .resolve_gateway_info(address)
                    .await
                {
                    Ok(Some(res)) => {
                        _ = self.insert(res.clone()).await;
                        Ok(res)
                    }
                    Ok(None) => Err(GatewayCacheError::GatewayNotFound(address.clone())),
                    Err(err) => {
                        metrics::increment_counter!("oracles_iot_verifier_config_service_error");
                        Err(GatewayCacheError::IotConfigClient(err))
                    }
                }
            }
        }
    }

    pub async fn insert(&self, gateway: GatewayInfo) -> anyhow::Result<()> {
        // add some jitter to the ttl
        let max_jitter = (CACHE_TTL.as_secs() * CACHE_TTL_JITTER_PERCENT) / 100;
        let jitter = OsRng.gen_range(0..=max_jitter);
        let ttl = CACHE_TTL + Duration::from_secs(jitter);
        self.cache
            .insert(gateway.address.clone(), gateway, ttl)
            .await;
        Ok(())
    }
}
