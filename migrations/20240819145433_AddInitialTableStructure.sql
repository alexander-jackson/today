CREATE TABLE item (
  id BIGINT GENERATED ALWAYS AS IDENTITY,
  item_uid UUID NOT NULL,
  content TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL,

  CONSTRAINT pk_item PRIMARY KEY (id),
  CONSTRAINT uk_item_item_uid UNIQUE (item_uid)
);

CREATE INDEX idx_item_created_at_desc ON item (created_at DESC);

CREATE TABLE item_event_type (
  id BIGINT GENERATED ALWAYS AS IDENTITY,
  name TEXT NOT NULL,

  CONSTRAINT pk_item_event_type PRIMARY KEY (id),
  CONSTRAINT uk_item_event_type_name UNIQUE (name)
);

INSERT INTO item_event_type (name) VALUES ('Checked'), ('Unchecked');

CREATE TABLE item_event (
  id BIGINT GENERATED ALWAYS AS IDENTITY,
  item_id BIGINT NOT NULL,
  event_type_id BIGINT NOT NULL,
  occurred_at TIMESTAMP NOT NULL,

  CONSTRAINT pk_item_event PRIMARY KEY (id),
  CONSTRAINT fk_item_event_item FOREIGN KEY (item_id) REFERENCES item (id)
);

CREATE INDEX idx_item_event_occurred_at_desc ON item_event (occurred_at DESC);
