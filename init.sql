CREATE TABLE clients (
    id SERIAL PRIMARY KEY,
    limit_value INTEGER NOT NULL,
    balance INTEGER NOT NULL
);

CREATE TABLE transactions (
    id SERIAL PRIMARY KEY,
    client_id INTEGER REFERENCES clients,
    value INTEGER NOT NULL,
    tran_type CHAR(1) NOT NULL,
    description VARCHAR(10) NOT NULL,
    created_at VARCHAR(32) NOT NULL
);

CREATE INDEX index_client_id_transactions ON transactions (client_id);

DO $$
BEGIN
INSERT INTO clients (limit_value, balance)
VALUES
    (100000, 0),
    (80000, 0),
    (1000000, 0),
    (10000000, 0),
    (500000, 0);
END;
$$