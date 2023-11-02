-- This table stores user roles in the application.

-- Create a table.
CREATE TABLE role(
    role_id SERIAL PRIMARY KEY,
    title VARCHAR(50) NOT NULL,
    role_description TEXT NOT NULL
); -- end create table role


-- Insert some roles.
INSERT INTO role
(title, role_description)
VALUES
('user', 'General user'),
('admin', 'A user that is able to manage general users'),
('superuser', 'A user that has all the rights in the application');