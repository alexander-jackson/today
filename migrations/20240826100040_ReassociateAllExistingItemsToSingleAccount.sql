-- Add the new column
ALTER TABLE item
ADD COLUMN account_id BIGINT;

-- Update all the rows
UPDATE item
SET account_id = (
  SELECT id
  FROM account
  ORDER BY created_at
  LIMIT 1
);

-- Create the foreign key
ALTER TABLE item
ADD CONSTRAINT fk_item_account_id
FOREIGN KEY (account_id)
REFERENCES account (id);

-- Not-null it
ALTER TABLE item
ALTER COLUMN account_id SET NOT NULL;
