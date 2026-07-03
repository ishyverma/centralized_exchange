use actix_web::{web, HttpResponse};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::WalletError;
use crate::models::*;
use backpack_common::error::ApiError;

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
    let balances = match row {
        Some(r) => vec![BalanceResponse::from(r)],
        None => vec![BalanceResponse {
            asset: asset.to_string(),
            free: "0".to_string(),
            locked: "0".to_string(),
        }],
    };
    Ok(HttpResponse::Ok().json(BalanceListResponse { balances }))
}

pub async fn reserve_balance(
    db: web::Data<DbPool>,
    body: web::Json<ReserveRequest>,
) -> Result<HttpResponse, WalletError> {
    let amount = Decimal::try_from(
        body.amount
            .parse::<f64>()
            .map_err(|_| WalletError(ApiError::ValidationError("Invalid amount".into())))?,
    )
    .map_err(|_| WalletError(ApiError::ValidationError("Invalid amount".into())))?;

    db.reserve_balance(body.user_id, &body.asset, amount, body.reference_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "reserved"})))
}

pub async fn release_balance(
    db: web::Data<DbPool>,
    body: web::Json<ReleaseRequest>,
) -> Result<HttpResponse, WalletError> {
    let amount = Decimal::try_from(
        body.amount
            .parse::<f64>()
            .map_err(|_| WalletError(ApiError::ValidationError("Invalid amount".into())))?,
    )
    .map_err(|_| WalletError(ApiError::ValidationError("Invalid amount".into())))?;

    db.release_balance(body.user_id, &body.asset, amount, body.reference_id)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "released"})))
}
