CREATE UNLOGGED TABLE clients (
    id SERIAL PRIMARY KEY,
    limit_value INTEGER NOT NULL,
    balance INTEGER NOT NULL
);

CREATE UNLOGGED TABLE transactions (
    id SERIAL PRIMARY KEY,
    client_id INTEGER REFERENCES clients,
    value INTEGER NOT NULL,
    tran_type CHAR(1) NOT NULL,
    description VARCHAR(10) NOT NULL,
    created_at VARCHAR(45) NOT NULL
);

CREATE INDEX index_id_clients ON clients (id);
CREATE INDEX index_client_id_created_at_transactions ON transactions (client_id, created_at DESC);
CREATE INDEX index_client_id_transactions ON transactions (client_id);

CREATE OR REPLACE FUNCTION process_transaction(
    p_client_id INTEGER,
    p_value INTEGER,
    p_tran_type CHAR(1),
    p_description VARCHAR(10),
    p_created_at VARCHAR(45)
) RETURNS TABLE (balance INT, limit_value INT) AS $$
DECLARE
new_balance INT;
BEGIN
INSERT INTO transactions (client_id, value, tran_type, description, created_at)
VALUES (p_client_id, p_value, p_tran_type, p_description, p_created_at);

UPDATE clients c
SET balance = CASE
                  WHEN p_tran_type = 'c' THEN c.balance + p_value
                  WHEN p_tran_type = 'd' AND c.balance - p_value >= -c.limit_value THEN c.balance - p_value
                  ELSE c.balance
    END
WHERE c.id = p_client_id
    RETURNING c.balance, c.limit_value INTO new_balance, limit_value;

RETURN QUERY SELECT new_balance, c.limit_value FROM clients c WHERE c.id = p_client_id;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION get_account_statement(client_id_param INTEGER)
RETURNS TABLE (
    client_id INTEGER,
    value INTEGER,
    tran_type CHAR(1),
    description VARCHAR(10),
    created_at VARCHAR(45),
    limit_value INTEGER,
    balance INTEGER
) AS $$
BEGIN
RETURN QUERY
SELECT
    c.id AS client_id,
    t.value,
    t.tran_type,
    t.description,
    t.created_at,
    c.limit_value,
    c.balance
FROM
    clients c
        LEFT JOIN
    transactions t ON c.id = t.client_id
WHERE
    c.id = client_id_param
ORDER BY
    t.created_at DESC
    LIMIT
        10;
END;
$$ LANGUAGE plpgsql;


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