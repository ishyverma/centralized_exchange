use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct BalanceQueryParams {
    pub asset: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Serialize)]
pub struct BalanceListResponse {
    pub balances: Vec<BalanceResponse>,
}

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub maker_commission: u32,
    pub taker_commission: u32,
    pub can_trade: bool,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    pub balances: Vec<BalanceResponse>,
}

#[derive(Debug, Deserialize)]
pub struct ReserveRequest {
    pub user_id: Uuid,
    pub asset: String,
    pub amount: String,
    pub reference_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseRequest {
    pub user_id: Uuid,
    pub asset: String,
    pub amount: String,
    pub reference_id: Option<Uuid>,
}

impl From<crate::db::BalanceRow> for BalanceResponse {
    fn from(row: crate::db::BalanceRow) -> Self {
        Self {
            asset: row.asset,
            free: row.available.to_string(),
            locked: row.reserved.to_string(),
        }
    }
}
