CREATE TABLE hotspot_threshold (
  hotspot_pubkey TEXT PRIMARY KEY NOT NULL,
  bytes_threshold BIGINT NOT NULL,
  subscriber_threshold INT NOT NULL,
  timestamp TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  created_at TIMESTAMPTZ DEFAULT NOW()
);
