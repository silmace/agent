use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum SocketIOError {
    #[serde(rename(serialize = "err_dns_lookup_failed"))]
    ErrDNSLookupFailed,
    #[serde(rename(serialize = "err_ping_failed"))]
    ErrPingFailed,
    #[serde(rename(serialize = "err_tcping_failed"))]
    ErrTCPingFailed,
    #[serde(rename(serialize = "err_http_failed"))]
    ErrHTTPFailed,
    #[serde(rename(serialize = "err_mtr_failed"))]
    ErrMTRFailed,
}
