-- PostgreSQL database load

CREATE DATABASE config_db
    WITH
    OWNER = postgres
    ENCODING = 'UTF-8'
    LC_COLLATE = 'en_US.utf8'
    LC_CTYPE = 'en_US.utf8'
    TABLESPACE = pg_default
    CONNECTION LIMIT = -1;

CREATE DATABASE metadata_db
    WITH
    OWNER = postgres
    ENCODING = 'UTF-8'
    LC_COLLATE = 'en_US.utf8'
    LC_CTYPE = 'en_US.utf8'
    TABLESPACE = pg_default
    CONNECTION LIMIT = -1;

CREATE DATABASE index_db
    WITH
    OWNER = postgres
    ENCODING = 'UTF-8'
    LC_COLLATE = 'en_US.utf8'
    LC_CTYPE = 'en_US.utf8'
    TABLESPACE = pg_default
    CONNECTION LIMIT = -1;

CREATE DATABASE verifier_db
    WITH
    OWNER = postgres
    ENCODING = 'UTF-8'
    LC_COLLATE = 'en_US.utf8'
    LC_CTYPE = 'en_US.utf8'
    TABLESPACE = pg_default
    CONNECTION LIMIT = -1;

CREATE DATABASE packet_verifier_db
    WITH
    OWNER = postgres
    ENCODING = 'UTF-8'
    LC_COLLATE = 'en_US.utf8'
    LC_CTYPE = 'en_US.utf8'
    TABLESPACE = pg_default
    CONNECTION LIMIT = -1;

-- PostgreSQL database load complete
