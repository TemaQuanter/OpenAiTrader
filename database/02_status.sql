-- This table contains all possible user statuses.

-- Create table.
CREATE TABLE status(
    status_id SERIAL PRIMARY KEY,
    title VARCHAR(50) NOT NULL,
    status_description TEXT NOT NULL
); -- end create table status

-- Insert some statuses.
INSERT INTO status
(title, status_description)
VALUES
('pending', 'A user is awaiting an approval.'),
('rejected', 'A user is rejected and cannot use the bot.'),
('approved', 'A user is approved to use the bot.'),
('banned', 'A user cannot use the bot anymore.');