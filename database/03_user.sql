-- This table stores some information about an application user
-- and their settings.

-- Create table.
CREATE TABLE users(
    -- A unique identifier of a user in telegram.
    telegram_id BIGINT PRIMARY KEY,
    -- A unique username in telegram.
    telegram_username VARCHAR(100),
    -- Indicates whether or not a user is approved/banned/rejected etc.
    status_id INT REFERENCES status (status_id) NOT NULL,
    -- Exchange API key of a user.
    exchange_api_key VARCHAR(200),
    -- Exchange API secret of a user.
    exchange_api_secret VARCHAR(200),
    -- The deposit that is used to calculate a volume per trade from.
    deposit VARCHAR(20),
    -- The risk (in %) that is used  to calculate a volume per trade from.
    risk VARCHAR(3),
    -- Whether or not bot is allowed to trade for this user.
    bot_active BOOLEAN NOT NULL
); -- end create table user