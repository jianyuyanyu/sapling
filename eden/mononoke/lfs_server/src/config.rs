/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::collections::BTreeSet;
use std::num::NonZeroU16;
use std::str::FromStr;

use anyhow::Context;
use anyhow::Error;
use anyhow::bail;
use gotham_ext::middleware::PostResponseConfig;
use permission_checker::MononokeIdentitySet;
use rate_limiting::LoadShedLimit;
use serde::Deserialize;
use serde::Serialize;
use serde::de::Deserializer;
use serde::de::Error as _;
use serde::ser::Serializer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectPopularity {
    /// SCS counter category to use for blob popularity.
    pub category: String,
    /// How long (in seconds) to lookback
    pub window: u32,
    /// Objects whose sum of downloads exceeds the threshold during the window will not be
    /// consistently-routed. This ensures the full pool of servers can be used to serve very
    /// popular blobs.
    pub thresholds: Vec<ConsistentRoutingRing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistentRoutingRing {
    pub threshold: u64,
    pub mode: ConsistentRoutingRingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsistentRoutingRingMode {
    Num { tasks_per_content: NonZeroU16 },
    All,
}

impl TryFrom<lfs_server_config::ConsistentRoutingRingMode> for ConsistentRoutingRingMode {
    type Error = Error;

    fn try_from(value: lfs_server_config::ConsistentRoutingRingMode) -> Result<Self, Self::Error> {
        let mode = match value {
            lfs_server_config::ConsistentRoutingRingMode::num(t) => {
                let tasks_per_content = t
                    .tasks_per_content
                    .try_into()
                    .with_context(|| "tasks_per_content is < 0")?;
                ConsistentRoutingRingMode::Num {
                    tasks_per_content: NonZeroU16::new(tasks_per_content)
                        .with_context(|| "tasks_per_content is 0")?,
                }
            }
            lfs_server_config::ConsistentRoutingRingMode::all(_) => ConsistentRoutingRingMode::All,
            lfs_server_config::ConsistentRoutingRingMode::UnknownField(_) => {
                bail!("Unknown field in ConsistentRoutingRingMode")
            }
        };
        Ok(mode)
    }
}

impl TryFrom<lfs_server_config::ConsistentRoutingRing> for ConsistentRoutingRing {
    type Error = Error;

    fn try_from(value: lfs_server_config::ConsistentRoutingRing) -> Result<Self, Self::Error> {
        let threshold = value
            .threshold
            .try_into()
            .with_context(|| format!("Invalid threshold: {:?}", value.threshold))?;

        let mode = value.mode.try_into().context("Invalid mode")?;

        Ok(Self { threshold, mode })
    }
}

impl TryFrom<lfs_server_config::ObjectPopularity> for ObjectPopularity {
    type Error = Error;

    fn try_from(value: lfs_server_config::ObjectPopularity) -> Result<Self, Self::Error> {
        let window = value
            .window
            .try_into()
            .with_context(|| format!("Invalid window: {:?}", value.window))?;

        let thresholds = value
            .thresholds
            .clone()
            .into_iter()
            .map(|l| l.try_into())
            .collect::<Result<Vec<_>, _>>()
            .context("Invalid routing thresholds")?;

        Ok(Self {
            category: value.category,
            window,
            thresholds,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub raw_server_config: lfs_server_config::LfsServerConfig,
    loadshedding_limits: Vec<LoadShedLimit>,
    object_popularity: Option<ObjectPopularity>,
    disable_compression_identities: Vec<MononokeIdentitySet>,
}

impl TryFrom<lfs_server_config::LfsServerConfig> for ServerConfig {
    type Error = Error;

    fn try_from(value: lfs_server_config::LfsServerConfig) -> Result<Self, Error> {
        let loadshedding_limits = value
            .loadshedding_limits
            .clone()
            .into_iter()
            .map(|l| l.try_into())
            .collect::<Result<Vec<_>, _>>()
            .context("Invalid loadshedding config")?;

        let object_popularity = value
            .object_popularity
            .as_ref()
            .map(|o| o.clone().try_into())
            .transpose()
            .with_context(|| "Invalid object popularity")?;

        let mut disable_compression_identities: Vec<MononokeIdentitySet> = Vec::new();
        for list in value.disable_compression_identities.iter() {
            let idents = list
                .iter()
                .map(|i| FromStr::from_str(i))
                .collect::<Result<BTreeSet<_>, _>>()?;
            disable_compression_identities.push(idents);
        }

        Ok(Self {
            raw_server_config: value,
            loadshedding_limits,
            object_popularity,
            disable_compression_identities,
        })
    }
}

impl<'de> Deserialize<'de> for ServerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = lfs_server_config::LfsServerConfig::deserialize(deserializer)?;
        let config = Self::try_from(raw).map_err(|e| D::Error::custom(format!("{:?}", e)))?;
        Ok(config)
    }
}

impl Serialize for ServerConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        lfs_server_config::LfsServerConfig::serialize(&self.raw_server_config, serializer)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        let raw_server_config = lfs_server_config::LfsServerConfig {
            track_bytes_sent: false,
            enable_consistent_routing: false,
            disable_hostname_logging: false,
            enforce_acl_check: false,
            loadshedding_limits: vec![],
            object_popularity: None,
            disable_compression: false,
            disable_compression_identities: vec![],
            enforce_authentication: false,
        };

        Self {
            raw_server_config,
            loadshedding_limits: vec![],
            object_popularity: None,
            disable_compression_identities: vec![],
        }
    }
}

impl ServerConfig {
    pub fn track_bytes_sent(&self) -> bool {
        self.raw_server_config.track_bytes_sent
    }
    pub fn enable_consistent_routing(&self) -> bool {
        self.raw_server_config.enable_consistent_routing
    }
    pub fn disable_hostname_logging(&self) -> bool {
        self.raw_server_config.disable_hostname_logging
    }
    pub fn loadshedding_limits(&self) -> Vec<LoadShedLimit> {
        self.loadshedding_limits.clone()
    }
    pub fn enforce_acl_check(&self) -> bool {
        self.raw_server_config.enforce_acl_check
    }
    pub fn enforce_authentication(&self) -> bool {
        self.raw_server_config.enforce_authentication
    }
    pub fn object_popularity(&self) -> Option<&ObjectPopularity> {
        self.object_popularity.as_ref()
    }
    #[cfg(test)]
    pub fn object_popularity_mut(&mut self) -> &mut Option<ObjectPopularity> {
        &mut self.object_popularity
    }
    pub fn disable_compression(&self) -> bool {
        self.raw_server_config.disable_compression
    }
    pub fn disable_compression_identities(&self) -> &Vec<MononokeIdentitySet> {
        &self.disable_compression_identities
    }
    #[cfg(test)]
    pub fn disable_compression_identities_mut(&mut self) -> &mut Vec<MononokeIdentitySet> {
        &mut self.disable_compression_identities
    }
}

impl PostResponseConfig for ServerConfig {
    fn resolve_hostname(&self) -> bool {
        !self.disable_hostname_logging()
    }
}
