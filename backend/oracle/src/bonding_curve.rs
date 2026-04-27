use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondingCurve {
    pub reserve_balance: f64,
    pub supply: f64,
    pub reserve_ratio: f64, // Connector Weight (e.g., 0.5 for 50%)
}

impl BondingCurve {
    pub fn new(reserve: f64, supply: f64, ratio: f64) -> Self {
        Self {
            reserve_balance: reserve,
            supply,
            reserve_ratio: ratio,
        }
    }

    /// Calculate tokens received for a given reserve deposit (e.g., USDC -> Token)
    pub fn calculate_purchase_return(&self, deposit_amount: f64) -> f64 {
        if deposit_amount <= 0.0 { return 0.0; }
        
        // Bancor Formula: T = S * ((1 + dR/R)^F - 1)
        self.supply * ((1.0 + deposit_amount / self.reserve_balance).powf(self.reserve_ratio) - 1.0)
    }

    /// Calculate reserve tokens received for selling supply tokens (e.g., Token -> USDC)
    pub fn calculate_sale_return(&self, sell_amount: f64) -> f64 {
        if sell_amount <= 0.0 || sell_amount >= self.supply { return 0.0; }

        // Bancor Formula: dR = R * (1 - (1 - dT/S)^(1/F))
        self.reserve_balance * (1.0 - (1.0 - sell_amount / self.supply).powf(1.0 / self.reserve_ratio))
    }

    /// Current instantaneous price
    pub fn current_price(&self) -> f64 {
        if self.supply <= 0.0 { return 0.0; }
        self.reserve_balance / (self.supply * self.reserve_ratio)
    }

    /// Simulated impact of a trade
    pub fn simulate_trade(&self, amount: f64, is_buy: bool) -> TradeImpact {
        let current_price = self.current_price();
        
        let (output_amount, new_reserve, new_supply) = if is_buy {
            let received = self.calculate_purchase_return(amount);
            (received, self.reserve_balance + amount, self.supply + received)
        } else {
            let received = self.calculate_sale_return(amount);
            (received, self.reserve_balance - received, self.supply - amount)
        };

        let new_price = new_reserve / (new_supply * self.reserve_ratio);
        let average_price = amount / output_amount; // amount of reserve per unit of supply token
        let price_impact = (new_price - current_price) / current_price;
        let slippage = (average_price - current_price).abs() / current_price;

        TradeImpact {
            input_amount: amount,
            output_amount,
            current_price,
            new_price,
            price_impact,
            slippage,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeImpact {
    pub input_amount: f64,
    pub output_amount: f64,
    pub current_price: f64,
    pub new_price: f64,
    pub price_impact: f64,
    pub slippage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bonding_curve_math() {
        let curve = BondingCurve::new(1000.0, 10000.0, 0.5);
        let price = curve.current_price();
        assert_eq!(price, 0.2); // 1000 / (10000 * 0.5) = 0.2

        let impact = curve.simulate_trade(100.0, true);
        assert!(impact.new_price > impact.current_price);
        assert!(impact.output_amount > 0.0);
    }
}
