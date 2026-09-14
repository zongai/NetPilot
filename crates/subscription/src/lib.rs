//! Airport subscription pipeline (S11, NP-121…NP-144).
//!
//! Integrates with existing `ProxyProfile` / `ProxyGroup`; no competing model.

#![forbid(unsafe_code)]

mod cache;
mod decoder;
mod diagnose;
mod fetcher;
mod filter;
mod groups;
mod http;
mod manager;
mod merge;
mod metadata;
mod node_fingerprint;
mod normalizer;
mod parsers;
mod pipeline;
mod policy;
mod profile;
mod rename;
mod rollback;
mod transport_map;
mod validate;

pub use cache::{CacheEntry, SubscriptionCache};
pub use decoder::{detect_and_decode, DecodeError, DecodedBody};
pub use diagnose::{diagnose_failure, DiagnoseReport, DiagnoseStage};
#[cfg(feature = "real-http")]
pub use fetcher::UreqFetcher;
pub use fetcher::{FetchError, FetchRequest, FetchResponse, MockFetcher, SubscriptionFetcher};
pub use filter::{apply_filters, FilterRule};
pub use groups::group_from_subscription;
pub use http::{HttpRequestHeaders, DEFAULT_USER_AGENT};
pub use manager::{SubscriptionError, SubscriptionManager};
pub use merge::{merge_profiles, MergeConflict};
pub use metadata::{parse_userinfo_header, SubscriptionUserinfo};
pub use node_fingerprint::{fingerprint, merge_duplicates};
pub use normalizer::normalize_nodes;
pub use parsers::{parse_clash_yaml, parse_singbox_json, parse_uri_list, ParsedNode};
pub use pipeline::{run_subscription_pipeline, PipelineResult};
pub use policy::{UpdatePolicy, UpdateTrigger};
pub use profile::{SubscriptionProfile, SubscriptionState};
pub use rename::{apply_rename, RenameRule};
pub use rollback::RollbackGuard;
pub use transport_map::map_transport_params;
pub use validate::{validate_subscription_security, SecurityIssue};

pub const CRATE_NAME: &str = "netpilot-subscription";
