use actix_web::{web, HttpResponse};
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::WalletError;
use crate::models::*;

pub async fn get_account(
    db: web::Data<DbPool>,
    user_id: web::ReqData<Uuid>,
) -> Result<HttpResponse, WalletError> {
    let uid = *user_id;
    let rows = db.list_balances(uid).await?;
    let balances: Vec<BalanceResponse> = rows.into_iter().map(BalanceResponse::from).collect();

    Ok(HttpResponse::Ok().json(AccountResponse {
        maker_commission: 10,
        taker_commission: 10,
        can_trade: true,
        can_withdraw: true,
        can_deposit: true,
        balances,
    }))
}

pub async fn get_balance(
    db: web::Data<DbPool>,
    user_id: web::ReqData<Uuid>,
    query: web::Query<BalanceQueryParams>,
) -> Result<HttpResponse, WalletError> {
    let uid = *user_id;
    let asset = query.asset.as_deref().unwrap_or("USDT");

    let row = db.get_balance(uid, asset).await?;
    match row {
        Some(r) => Ok(HttpResponse::Ok().json(BalanceResponse::from(r))),
        None => Ok(HttpResponse::Ok().json(BalanceResponse {
            asset: asset.to_string(),
            free: "0".to_string(),
            locked: "0".to_string(),
        })),
    }
}
