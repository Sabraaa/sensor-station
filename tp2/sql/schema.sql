-- Reference table for a new installation in the selected database.
-- The collector never executes DDL or modifies an existing table.
CREATE TABLE IF NOT EXISTS measurements (
    id INT NOT NULL AUTO_INCREMENT,
    `timestamp` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    temperature FLOAT NULL DEFAULT NULL,
    humidity FLOAT NULL DEFAULT NULL,
    pressure FLOAT NULL DEFAULT NULL,
    co2 FLOAT NULL DEFAULT NULL,
    no2 FLOAT NULL DEFAULT NULL,
    co FLOAT NULL DEFAULT NULL,
    nh3 FLOAT NULL DEFAULT NULL,
    PRIMARY KEY (id)
) ENGINE = InnoDB;
