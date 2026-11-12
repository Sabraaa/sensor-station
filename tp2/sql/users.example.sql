-- Copy outside Git, replace passwords/usernames to match the local config.json,
-- then execute as a MariaDB administrator. This is not a production secret file.
-- Existing accounts/passwords are deliberately not changed by this template.
CREATE USER IF NOT EXISTS 'umwelt_writer'@'localhost' IDENTIFIED BY 'REPLACE_WRITER_PASSWORD';
CREATE USER IF NOT EXISTS 'umwelt_reader'@'localhost' IDENTIFIED BY 'REPLACE_READER_PASSWORD';
GRANT INSERT ON umwelt_db.measurements TO 'umwelt_writer'@'localhost';
GRANT SELECT ON umwelt_db.measurements TO 'umwelt_reader'@'localhost';
