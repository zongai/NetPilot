//! DNS metrics and diagnostics (NP-083).

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DnsMetrics {
    pub queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub nxdomain: u64,
    pub transport_errors: u64,
    pub blocked: u64,
    pub fakeip_allocs: u64,
}

impl DnsMetrics {
    pub fn record_query(&mut self) {
        self.queries = self.queries.saturating_add(1);
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits = self.cache_hits.saturating_add(1);
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses = self.cache_misses.saturating_add(1);
    }

    pub fn record_nxdomain(&mut self) {
        self.nxdomain = self.nxdomain.saturating_add(1);
    }

    pub fn record_transport_error(&mut self) {
        self.transport_errors = self.transport_errors.saturating_add(1);
    }

    pub fn record_blocked(&mut self) {
        self.blocked = self.blocked.saturating_add(1);
    }

    pub fn record_fakeip(&mut self) {
        self.fakeip_allocs = self.fakeip_allocs.saturating_add(1);
    }

    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "queries={} hits={} misses={} nx={} transport_err={} blocked={} fakeip={}",
            self.queries,
            self.cache_hits,
            self.cache_misses,
            self.nxdomain,
            self.transport_errors,
            self.blocked,
            self.fakeip_allocs
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_and_summary() {
        let mut m = DnsMetrics::default();
        m.record_query();
        m.record_cache_hit();
        m.record_cache_miss();
        assert!((m.cache_hit_ratio() - 0.5).abs() < f64::EPSILON);
        assert!(m.summary().contains("queries=1"));
    }
}
