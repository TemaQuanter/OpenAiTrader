-- This table is used to store the trusted telegram channel history
-- to recover chains of messages that reply on one another.

-- Create table.
CREATE TABLE channel_history(
    message_id INT PRIMARY KEY,
    response_message_id INT REFERENCES channel_history (message_id),
    message_text TEXT,
    message_timestamp TIMESTAMP WITH TIME ZONE NOT NULL
); -- end create table channel_history