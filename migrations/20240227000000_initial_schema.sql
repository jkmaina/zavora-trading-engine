-- Create accounts table
CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS accounts_user_id_idx ON accounts(user_id);

-- Create balances table
CREATE TABLE IF NOT EXISTS balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES accounts(id),
    asset TEXT NOT NULL,
    total TEXT NOT NULL,
    available TEXT NOT NULL,
    locked TEXT NOT NULL DEFAULT '0',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(account_id, asset)
);

CREATE INDEX IF NOT EXISTS balances_account_id_idx ON balances(account_id);
CREATE INDEX IF NOT EXISTS balances_asset_idx ON balances(asset);

-- Create markets table
CREATE TABLE IF NOT EXISTS markets (
    id TEXT PRIMARY KEY,
    base_asset TEXT NOT NULL,
    quote_asset TEXT NOT NULL,
    min_price TEXT NOT NULL,
    max_price TEXT NOT NULL,
    tick_size TEXT NOT NULL,
    min_quantity TEXT NOT NULL,
    max_quantity TEXT NOT NULL,
    step_size TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create orders table
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES accounts(id),
    market_id TEXT NOT NULL REFERENCES markets(id),
    side SMALLINT NOT NULL,
    order_type SMALLINT NOT NULL,
    price TEXT,
    quantity TEXT NOT NULL,
    filled_quantity TEXT NOT NULL DEFAULT '0',
    status SMALLINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS orders_account_id_idx ON orders(account_id);
CREATE INDEX IF NOT EXISTS orders_market_id_idx ON orders(market_id);
CREATE INDEX IF NOT EXISTS orders_status_idx ON orders(status);

-- Create trades table
CREATE TABLE IF NOT EXISTS trades (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    market_id TEXT NOT NULL REFERENCES markets(id),
    maker_order_id UUID NOT NULL REFERENCES orders(id),
    taker_order_id UUID NOT NULL REFERENCES orders(id),
    price TEXT NOT NULL,
    quantity TEXT NOT NULL,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS trades_market_id_idx ON trades(market_id);
CREATE INDEX IF NOT EXISTS trades_maker_order_id_idx ON trades(maker_order_id);
CREATE INDEX IF NOT EXISTS trades_taker_order_id_idx ON trades(taker_order_id);
CREATE INDEX IF NOT EXISTS trades_executed_at_idx ON trades(executed_at);

-- Create market_summaries table
CREATE TABLE IF NOT EXISTS market_summaries (
    market_id TEXT PRIMARY KEY REFERENCES markets(id),
    open_price TEXT NOT NULL,
    high_price TEXT NOT NULL,
    low_price TEXT NOT NULL,
    close_price TEXT NOT NULL,
    volume TEXT NOT NULL DEFAULT '0',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create order_books table
CREATE TABLE IF NOT EXISTS order_books (
    market_id TEXT PRIMARY KEY REFERENCES markets(id),
    data JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers for updated_at columns
CREATE TRIGGER update_accounts_updated_at
BEFORE UPDATE ON accounts
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_balances_updated_at
BEFORE UPDATE ON balances
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_markets_updated_at
BEFORE UPDATE ON markets
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_orders_updated_at
BEFORE UPDATE ON orders
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();