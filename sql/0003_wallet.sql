CREATE TABLE balances (
    user_id UUID NOT NULL REFERENCES users(id),
    asset VARCHAR(20) NOT NULL,
    total NUMERIC(20,8) NOT NULL DEFAULT 0,
    available NUMERIC(20,8) NOT NULL DEFAULT 0,
    reserved NUMERIC(20,8) NOT NULL DEFAULT 0,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, asset)
);

CREATE TABLE balance_events (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    asset VARCHAR(20) NOT NULL,
    event_type VARCHAR(20) NOT NULL,
    amount NUMERIC(20,8) NOT NULL,
    balance_before_total NUMERIC(20,8) NOT NULL,
    balance_after_total NUMERIC(20,8) NOT NULL,
    reference_id UUID,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE deposits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    asset VARCHAR(20) NOT NULL,
    amount NUMERIC(20,8) NOT NULL,
    tx_hash VARCHAR(255) UNIQUE NOT NULL,
    confirmations INT NOT NULL DEFAULT 0,
    required_confirmations INT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    detected_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE withdrawals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    asset VARCHAR(20) NOT NULL,
    amount NUMERIC(20,8) NOT NULL,
    fee NUMERIC(20,8) NOT NULL DEFAULT 0,
    address VARCHAR(255) NOT NULL,
    tx_hash VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    rejection_reason TEXT,
    requested_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_balance_events_user_id ON balance_events(user_id);
CREATE INDEX idx_deposits_user_id ON deposits(user_id);
CREATE INDEX idx_deposits_status ON deposits(status);
CREATE INDEX idx_withdrawals_user_id ON withdrawals(user_id);
CREATE INDEX idx_withdrawals_status ON withdrawals(status);

INSERT INTO balances (user_id, asset, total, available, reserved)
SELECT id, 'USDT', 1000000, 1000000, 0 FROM users
ON CONFLICT DO NOTHING;
