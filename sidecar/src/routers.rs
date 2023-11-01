use std::sync::Arc;

use axum::http::StatusCode;
use axum::{Extension, Json};
use prometheus::Encoder;
use tokio::sync::{Mutex, RwLock};
use tracing::error;

use crate::types::{MembershipChange, StatePayload};
use crate::utils::{check_backup_volume, check_data_volume};
use crate::xline::XlineHandle;

/// metrics handler
#[allow(clippy::unused_async)] // require by axum
pub(crate) async fn metrics(Extension(registry): Extension<prometheus::Registry>) -> String {
    let mut buf1 = Vec::new();
    let encoder = prometheus::TextEncoder::new();
    let metric_families = registry.gather();
    if let Err(err) = encoder.encode(&metric_families, &mut buf1) {
        error!("failed to encode custom metrics: {}", err);
        return String::new();
    }
    let mut res = String::from_utf8(buf1).unwrap_or_default();
    let mut buf2 = Vec::new();
    if let Err(err) = encoder.encode(&prometheus::gather(), &mut buf2) {
        error!("failed to encode prometheus metrics: {}", err);
        return String::new();
    }
    res.push_str(&String::from_utf8_lossy(&buf2));
    res
}

/// Return the current health condition according to the current node's storage volume and network status
/// The network status is verified upon returning the HTTP response.
#[allow(clippy::unused_async)] // This is required in axum
pub(crate) async fn health() -> StatusCode {
    if !check_data_volume() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if !check_backup_volume() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

/// Backup hook
pub(crate) async fn backup(Extension(handle): Extension<Arc<RwLock<XlineHandle>>>) -> StatusCode {
    if handle.read().await.backup().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
}

/// State route
pub(crate) async fn state(
    Extension(state): Extension<Arc<Mutex<StatePayload>>>,
) -> (StatusCode, Json<StatePayload>) {
    let state = state.lock().await;
    let payload = state.clone();
    (StatusCode::OK, Json(payload))
}

/// Proactively initiate membership change.
#[allow(clippy::unused_async)] // TODO remove when it is implemented
pub(crate) async fn membership(_change: Json<MembershipChange>) -> StatusCode {
    // TODO handle proactively membership change in server handle
    StatusCode::OK
}
