use actix_web::{web, HttpResponse};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::db::DbPool;
use crate::engine_client::EngineClient;
use crate::error::OrderError;
use crate::models::*;
use backpack_common::error::ApiError;

pub async fn place_order(
    db: web::Data<DbPool>,
    engine: web::Data<EngineClient>,
    user_id: web::ReqData<Uuid>,
    body: web::Json<PlaceOrderRequest>,
) -> Result<HttpResponse, OrderError> {
    let uid = *user_id;
    let symbol = body.symbol.to_uppercase();

    let side = body.side.to_uppercase();
    if side != "BUY" && side != "SELL" {
        return Err(OrderError(ApiError::ValidationError(
            "Side must be BUY or SELL".into(),
        )));
    }

    let order_type = body.order_type.to_uppercase();
    if order_type != "LIMIT" && order_type != "MARKET" {
        return Err(OrderError(ApiError::ValidationError(
            "Order type must be LIMIT or MARKET".into(),
        )));
    }

    if body.quantity <= Decimal::ZERO {
        return Err(OrderError(ApiError::ValidationError(
            "Quantity must be positive".into(),
        )));
    }

    if order_type == "LIMIT" && (body.price.is_none() || body.price.unwrap() <= Decimal::ZERO) {
        return Err(OrderError(ApiError::ValidationError(
            "Limit order requires a valid price".into(),
        )));
    }

    let time_in_force = body.time_in_force.to_uppercase();
    if !["GTC", "IOC", "FOK", "GTD"].contains(&time_in_force.as_str()) {
        return Err(OrderError(ApiError::ValidationError(
            "Invalid time_in_force".into(),
        )));
    }

    let client_order_id = body.new_client_order_id.as_deref();

    let order_row = db
        .create_order(crate::db::CreateOrderParams {
            user_id: uid,
            symbol: &symbol,
            side: &side,
            order_type: &order_type,
            price: body.price,
            quantity: body.quantity,
            time_in_force: &time_in_force,
            client_order_id,
        })
        .await?;

    let matches = engine
        .process_order(&order_row)
        .await
        .map_err(|e| OrderError(ApiError::Internal(format!("Engine error: {}", e))))?;

    let mut taker_total_filled = Decimal::ZERO;

    for m in &matches {
        taker_total_filled = m.taker_filled;

        if let Some(trade) = &m.trade {
            sqlx::query(
                "INSERT INTO trades (symbol, price, quantity, quote_quantity, buyer_order_id, seller_order_id, buyer_user_id, seller_user_id, taker_side, trade_time, match_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(&trade.symbol)
            .bind(trade.price)
            .bind(trade.quantity)
            .bind(trade.quote_quantity)
            .bind(trade.buyer_order_id)
            .bind(trade.seller_order_id)
            .bind(trade.buyer_user_id)
            .bind(trade.seller_user_id)
            .bind(&trade.taker_side)
            .bind(trade.trade_time)
            .bind(trade.match_id)
            .execute(&db.pool)
            .await
            .map_err(|e| OrderError(ApiError::Internal(format!("DB error: {}", e))))?;
        }
    }

    let order_qty = order_row.quantity;
    let taker_status = if taker_total_filled >= order_qty {
        "FILLED"
    } else if taker_total_filled > Decimal::ZERO {
        "PARTIALLY_FILLED"
    } else {
        "NEW"
    };

    if taker_total_filled > Decimal::ZERO {
        db.update_order_status(order_row.id, taker_status, taker_total_filled)
            .await?;
    }

    let latest = db.get_order(order_row.id, uid).await?.unwrap_or(order_row);

    Ok(HttpResponse::Ok().json(OrderResponse::from(latest)))
}

pub async fn get_order(
    db: web::Data<DbPool>,
    user_id: web::ReqData<Uuid>,
    query: web::Query<QueryOrderParams>,
) -> Result<HttpResponse, OrderError> {
    let uid = *user_id;

    let order = if let Some(order_id) = query.order_id {
        db.get_order(order_id, uid).await?
    } else if let Some(ref client_id) = query.orig_client_order_id {
        db.get_order_by_client_id(client_id, uid).await?
    } else {
        return Err(OrderError(ApiError::ValidationError(
            "orderId or origClientOrderId required".into(),
        )));
    };

    match order {
        Some(row) => Ok(HttpResponse::Ok().json(OrderResponse::from(row))),
        None => Err(OrderError(ApiError::OrderNotFound)),
    }
}

pub async fn cancel_order(
    db: web::Data<DbPool>,
    engine: web::Data<EngineClient>,
    user_id: web::ReqData<Uuid>,
    query: web::Query<CancelOrderParams>,
) -> Result<HttpResponse, OrderError> {
    let uid = *user_id;

    let order = if let Some(order_id) = query.order_id {
        db.cancel_order(order_id, uid).await?
    } else if let Some(ref client_id) = query.orig_client_order_id {
        let existing = db.get_order_by_client_id(client_id, uid).await?;
        match existing {
            Some(row) if row.status == "NEW" || row.status == "PARTIALLY_FILLED" => {
                db.cancel_order(row.id, uid).await?
            }
            Some(_) => {
                return Err(OrderError(ApiError::ValidationError(
                    "Order is not cancellable in current status".into(),
                )));
            }
            None => return Err(OrderError(ApiError::OrderNotFound)),
        }
    } else {
        return Err(OrderError(ApiError::ValidationError(
            "orderId or origClientOrderId required".into(),
        )));
    };

    match order {
        Some(row) => {
            let price = row.price.unwrap_or_default();
            engine
                .cancel_order(row.id, &row.symbol, &row.side, price)
                .await;
            Ok(HttpResponse::Ok().json(OrderResponse::from(row)))
        }
        None => Err(OrderError(ApiError::OrderNotFound)),
    }
}

pub async fn all_orders(
    db: web::Data<DbPool>,
    user_id: web::ReqData<Uuid>,
    query: web::Query<AllOrdersParams>,
) -> Result<HttpResponse, OrderError> {
    let uid = *user_id;
    let limit = query.limit.unwrap_or(500).min(1000);
    let offset = query.offset.unwrap_or(0);

    let orders = db
        .list_orders(uid, query.symbol.as_deref(), limit, offset)
        .await?;

    let response: Vec<OrderResponse> = orders.into_iter().map(OrderResponse::from).collect();
    Ok(HttpResponse::Ok().json(response))
}
