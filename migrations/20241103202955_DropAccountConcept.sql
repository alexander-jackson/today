-- Accounts are no longer needed, we have mTLS now
ALTER TABLE item
DROP COLUMN account_id;

DROP TABLE account;
