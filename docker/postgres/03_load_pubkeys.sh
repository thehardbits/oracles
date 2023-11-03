#!/bin/bash

psql -U postgres -d iot_config_db <<EOSQL
    INSERT INTO admin_keys (pubkey, key_type, inserted_at, updated_at)
    VALUES ('14qpSqtrquCjKFARFXnGfW1QE3L6KG2YWEZMi8CvZ5Yorz85Rrc', 'oracle', TIMESTAMP '2023-10-31T23:00:00.0+00', TIMESTAMP '2023-10-31T23:00:00.0+00'),
           ('13JqKYh8eUkRE2aQcTckBmBo82AhrHmE219ZBbraYTKKgGEh2QF', 'oracle', TIMESTAMP '2023-10-31T22:00:00.0+00', TIMESTAMP '2023-10-31T22:00:00.0+00'),
           ('141jjqmhrBAE3qtR3ggFwAVcd5StGDRpP6dRRFgsXC7rV32Kngt', 'administrator',	TIMESTAMP '2023-10-31T21:00:00.0+00', TIMESTAMP '2023-10-31T21:00:00.0+00');
EOSQL

psql -U postgres -d mobile_config_db <<EOSQL
    INSERT INTO registered_keys (pubkey, key_role, inserted_at, updated_at)
    VALUES ('14qpSqtrquCjKFARFXnGfW1QE3L6KG2YWEZMi8CvZ5Yorz85Rrc', 'oracle', TIMESTAMP '2023-10-31T23:00:00.0+00', TIMESTAMP '2023-10-31T23:00:00.0+00'),
           ('13JqKYh8eUkRE2aQcTckBmBo82AhrHmE219ZBbraYTKKgGEh2QF', 'oracle', TIMESTAMP '2023-10-31T22:00:00.0+00', TIMESTAMP '2023-10-31T22:00:00.0+00'),
           ('141jjqmhrBAE3qtR3ggFwAVcd5StGDRpP6dRRFgsXC7rV32Kngt', 'administrator',	TIMESTAMP '2023-10-31T21:00:00.0+00', TIMESTAMP '2023-10-31T21:00:00.0+00');
EOSQL
