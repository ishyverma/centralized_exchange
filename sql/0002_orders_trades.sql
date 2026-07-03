CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    symbol VARCHAR(20) NOT NULL,
    side VARCHAR(4) NOT NULL CHECK (side IN ('BUY', 'SELL')),
    order_type VARCHAR(20) NOT NULL,
    price NUMERIC(20,8),
    quantity NUMERIC(20,8) NOT NULL,
    filled_quantity NUMERIC(20,8) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'NEW' CHECK (status IN ('NEW', 'PARTIALLY_FILLED', 'FILLED', 'CANCELLED', 'REJECTED', 'EXPIRED')),
    time_in_force VARCHAR(3) NOT NULL DEFAULT 'GTC',
    client_order_id VARCHAR(64),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE trades (
    id BIGSERIAL PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL,
    price NUMERIC(20,8) NOT NULL,
    quantity NUMERIC(20,8) NOT NULL,
    quote_quantity NUMERIC(20,8) NOT NULL,
    buyer_order_id UUID NOT NULL,
    seller_order_id UUID NOT NULL,
    buyer_user_id UUID NOT NULL,
    seller_user_id UUID NOT NULL,
    taker_side VARCHAR(4) NOT NULL,
    trade_time TIMESTAMP WITH TIME ZONE NOT NULL,
    match_id BIGINT NOT NULL
);

CREATE INDEX idx_orders_user_id ON orders(user_id);
CREATE INDEX idx_orders_symbol_status ON orders(symbol, status);
CREATE INDEX idx_orders_created_at ON orders(created_at);
CREATE INDEX idx_orders_client_order_id ON orders(client_order_id);
CREATE INDEX idx_trades_symbol_time ON trades(symbol, trade_time);
CREATE INDEX idx_trades_buyer ON trades(buyer_user_id);
CREATE INDEX idx_trades_seller ON trades(seller_user_id);
