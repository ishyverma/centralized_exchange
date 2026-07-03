use actix_web::{web, HttpResponse};
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::db::DbPool;
use crate::error::MarketDataError;
use crate::models::*;
use backpack_common::error::ApiError;

pub async fn exchange_info() -> HttpResponse {
    HttpResponse::Ok().json(ExchangeInfoResponse {
        timezone: "UTC".to_string(),
        server_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        symbols: vec![SymbolInfo {
            symbol: "BTCUSDT".to_string(),
            status: "TRADING".to_string(),
            base_asset: "BTC".to_string(),
            quote_asset: "USDT".to_string(),
            order_types: vec![
                "LIMIT".to_string(),
                "MARKET".to_string(),
                "STOP_LOSS".to_string(),
            ],
            is_spot_trading_allowed: true,
        }],
    })
}

pub async fn get_recent_trades(
    db: web::Data<DbPool>,
    query: web::Query<TradesQueryParams>,
) -> Result<HttpResponse, MarketDataError> {
    let symbol = query.symbol.to_uppercase();
    let limit = query.limit.unwrap_or(500).min(1000);

    let trades = db.get_recent_trades(&symbol, limit).await?;
    let response: Vec<TradeResponse> = trades.into_iter().map(TradeResponse::from).collect();

    Ok(HttpResponse::Ok().json(response))
}

pub async fn get_ticker_24hr(
    db: web::Data<DbPool>,
    query: web::Query<TickerQueryParams>,
) -> Result<HttpResponse, MarketDataError> {
    let symbol = query
        .symbol
        .as_deref()
        .ok_or_else(|| MarketDataError(ApiError::ValidationError("symbol is required".into())))?
        .to_uppercase();

    let trades = db.get_trades_24hr(&symbol).await?;

    if trades.is_empty() {
        return Ok(HttpResponse::Ok().json(Ticker24hrResponse {
            symbol: symbol.clone(),
            price_change: "0".to_string(),
            price_change_percent: "0".to_string(),
            last_price: "0".to_string(),
            high_price: "0".to_string(),
            low_price: "0".to_string(),
            volume: "0".to_string(),
            quote_volume: "0".to_string(),
            count: 0,
        }));
    }

    let first_price = trades[0].price;
    let last_price = trades[trades.len() - 1].price;
    let mut high_price = trades[0].price;
    let mut low_price = trades[0].price;
    let mut volume = Decimal::ZERO;
    let mut quote_volume = Decimal::ZERO;

    for trade in &trades {
        if trade.price > high_price {
            high_price = trade.price;
        }
        if trade.price < low_price {
            low_price = trade.price;
        }
        volume += trade.quantity;
        quote_volume += trade.quote_quantity;
    }

    let price_change = last_price - first_price;
    let price_change_percent = if first_price != Decimal::ZERO {
        ((price_change / first_price) * Decimal::from_str("100").unwrap()).to_string()
    } else {
        "0".to_string()
    };

    Ok(HttpResponse::Ok().json(Ticker24hrResponse {
        symbol: symbol.clone(),
        price_change: price_change.to_string(),
        price_change_percent,
        last_price: last_price.to_string(),
        high_price: high_price.to_string(),
        low_price: low_price.to_string(),
        volume: volume.to_string(),
        quote_volume: quote_volume.to_string(),
        count: trades.len() as i64,
    }))
}

pub async fn get_ticker_price(
    db: web::Data<DbPool>,
    query: web::Query<TickerQueryParams>,
) -> Result<HttpResponse, MarketDataError> {
    match &query.symbol {
        Some(symbol) => {
            let sym = symbol.to_uppercase();
            let price = db.get_last_price(&sym).await?;
            Ok(HttpResponse::Ok().json(TickerPriceResponse {
                symbol: sym,
                price: price
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "0".to_string()),
            }))
        }
        None => Err(MarketDataError(ApiError::ValidationError(
            "symbol is required".into(),
        ))),
    }
}
