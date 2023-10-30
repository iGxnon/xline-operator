#![allow(dead_code)]

use operator_api::XlineConfig;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::ops::{Add, AddAssign, Div};
use std::path::PathBuf;
use std::time::Duration;

/// Sidecar operator config
#[derive(Debug, Clone)]
#[allow(clippy::exhaustive_structs)] // It is only constructed once
pub struct Config {
    /// Name of this node
    pub name: String,
    /// The cluster name
    pub cluster_name: String,
    /// Initial member config
    pub init_member: MemberConfig,
    /// Reconcile cluster interval
    pub reconcile_interval: Duration,
    /// The xline config
    pub xline: XlineConfig,
    /// The backend to run xline
    pub backend: BackendConfig,
    /// The sidecar monitor (operator) config, set to enable heartbeat
    pub monitor: Option<MonitorConfig>,
    /// Backup storage config
    pub backup: Option<BackupConfig>,
    /// The sidecar registry config, set to enable configuration discovery
    pub registry: Option<RegistryConfig>,
}

/// Monitor(Operator) config
#[derive(Debug, Clone)]
#[allow(clippy::exhaustive_structs)] // It is only constructed once
pub struct MonitorConfig {
    /// Monitor address
    pub monitor_addr: String,
    /// heartbeat interval
    pub heartbeat_interval: Duration,
}

/// Registry config
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum RegistryConfig {
    /// Statefulset mode
    Sts {
        /// Statefulset name
        name: String,
        /// Statefulset namespace
        namespace: String,
    },
    /// Http mode
    Http {
        /// Http server address
        server_addr: String,
    },
}

/// Member config
#[derive(Debug, Clone)]
#[allow(clippy::exhaustive_structs)] // It is only constructed once
pub struct MemberConfig {
    /// Nodes hosts, [pod_name]->[pod_host]
    pub members: HashMap<String, String>,
    /// The xline server port
    pub xline_port: u16,
    /// The sidecar web server port
    pub sidecar_port: u16,
}

/// Sidecar backend, it determinate how xline could be setup
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendConfig {
    /// K8s backend
    K8s {
        /// The pod name of this node
        pod_name: String,
        /// The xline container name, used to attach on it
        container_name: String,
        /// The namespace of this node
        namespace: String,
    },
    /// Local backend
    Local,
}

/// Backup storage config
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackupConfig {
    /// S3 storage
    S3 {
        /// S3 bucket name
        bucket: String,
    },
    /// PV storage
    PV {
        /// Mounted path of pv
        path: PathBuf,
    },
}

impl MemberConfig {
    /// Get the xline members
    pub(crate) fn xline_members(&self) -> HashMap<String, String> {
        self.members
            .clone()
            .into_iter()
            .map(|(name, host)| (name, format!("{host}:{}", self.xline_port)))
            .collect()
    }

    /// Get the sidecar members
    pub(crate) fn sidecar_members(&self) -> HashMap<String, String> {
        self.members
            .clone()
            .into_iter()
            .map(|(name, host)| (name, format!("{host}:{}", self.sidecar_port)))
            .collect()
    }

    /// Get the majority count
    pub(crate) fn majority_cnt(&self) -> usize {
        self.members.len().div(2).add(1)
    }

    /// Get the host
    pub(crate) fn get_host(&self, name: &str) -> Option<&String> {
        self.members.get(name)
    }
}

/// Sidecar operator state
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Hash)]
pub(crate) enum State {
    /// When this operator is trying to start it's kvserver
    Start,
    /// When this operator is pending on some confuse cluster status
    Pending,
    /// When this operator is working normally
    OK,
}

/// The state payload to expose states to other operators
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct StatePayload {
    /// Current state
    pub(crate) state: State,
    /// Current revision
    pub(crate) revision: i64,
}

/// The gathered states from sidecars
#[derive(Debug, Clone)]
pub(crate) struct StateStatus {
    /// A sidecar with highest revision is considered as "seeder".
    /// There could be more than one "seeder".
    pub(crate) seeder: String,
    /// State count, used to determine cluster status
    pub(crate) states: HashMap<State, usize>,
}

impl StateStatus {
    /// Gather status from sidecars
    pub(crate) async fn gather(sidecars: &HashMap<String, String>) -> anyhow::Result<Self> {
        use operator_api::consts::SIDECAR_STATE_ROUTE;

        let mut seeder = "";
        let mut max_rev = i64::MIN;
        let mut states = HashMap::<State, usize>::new();

        for (name, addr) in sidecars {
            let url = format!("http://{addr}{SIDECAR_STATE_ROUTE}");
            let state: StatePayload = reqwest::get(url).await?.json().await?;
            if (state.revision, name.as_str()) > (max_rev, seeder) {
                max_rev = state.revision;
                seeder = name;
            }
            let _ig = states
                .entry(state.state)
                .and_modify(|cnt| cnt.add_assign(1))
                .or_default();
        }
        Ok(Self {
            seeder: seeder.to_owned(),
            states,
        })
    }
}

/// The membership change request sent by other sidecar operators when they are shutting down
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct MembershipChange {
    /// The name of the sidecar operator
    pub(crate) name: String,
    /// The operation of this membership change request
    pub(crate) op: ChangeOP,
}

/// The change operation
#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum ChangeOP {
    /// Remove this member
    Remove,
    /// Add this member with an address
    Add(String),
}
