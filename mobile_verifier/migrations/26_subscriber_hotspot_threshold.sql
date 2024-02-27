create table hotspot_threshold (
  hotspot_pubkey TEXT primary key not null,
  bytes_threshold BIGINT not null,
  subscriber_threshold int not null,
  timestamp TIMESTAMPTZ not null,
  updated_at TIMESTAMPTZ default now(),
  created_at TIMESTAMPTZ default now()
);
