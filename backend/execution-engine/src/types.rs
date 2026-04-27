use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub String);

impl From<&str> for AssetId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for AssetId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSnapshot {
    pub pool_id: String,
    pub base_asset: AssetId,
    pub quote_asset: AssetId,
    pub base_reserve: f64,
    pub quote_reserve: f64,
    pub fee_bps: u32,
    pub updated_ledger: u64,
}

#[derive(Debug, Clone)]
pub struct EdgeView {
    pub pool_id: String,
    pub from: AssetId,
    pub to: AssetId,
    pub rate: f64,
    pub liquidity_cap: f64,
    pub fee_bps: u32,
    pub updated_ledger: u64,
}

impl PoolSnapshot {
    pub fn validate(&self) -> bool {
        self.base_reserve.is_finite()
            && self.quote_reserve.is_finite()
            && self.base_reserve > 0.0
            && self.quote_reserve > 0.0
            && self.fee_bps <= 10_000
    }

    pub fn directed_edges(&self) -> Option<[EdgeView; 2]> {
        if !self.validate() {
            return None;
        }

        let fee_factor = 1.0 - (self.fee_bps as f64 / 10_000.0);
        let liquidity_cap = self.base_reserve.min(self.quote_reserve);

        let forward_rate = (self.quote_reserve / self.base_reserve) * fee_factor;
        let reverse_rate = (self.base_reserve / self.quote_reserve) * fee_factor;

        Some([
            EdgeView {
                pool_id: self.pool_id.clone(),
                from: self.base_asset.clone(),
                to: self.quote_asset.clone(),
                rate: forward_rate,
                liquidity_cap,
                fee_bps: self.fee_bps,
                updated_ledger: self.updated_ledger,
            },
            EdgeView {
                pool_id: self.pool_id.clone(),
                from: self.quote_asset.clone(),
                to: self.base_asset.clone(),
                rate: reverse_rate,
                liquidity_cap,
                fee_bps: self.fee_bps,
                updated_ledger: self.updated_ledger,
            },
        ])
    }
}
