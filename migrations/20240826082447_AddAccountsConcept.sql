CREATE TABLE account (
  id BIGINT GENERATED ALWAYS AS IDENTITY,
  account_uid UUID NOT NULL,
  email_address TEXT NOT NULL,
  password TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL,

  CONSTRAINT pk_account PRIMARY KEY (id),
  CONSTRAINT uk_account_account_uid UNIQUE (account_uid)
);

CREATE UNIQUE INDEX uk_account_email_address_lower ON account (lower(email_address));
