-- PostgreSQL database dump

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

CREATE TABLE public.admin_keys (
    pubkey text NOT NULL,
    key_type public.key_type NOT NULL,
    inserted_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE public.admin_keys OWNER TO postgres;

COPY public.admin_keys (pubkey, key_type, inserted_at, updated_at) FROM stdin;
14qpSqtrquCjKFARFXnGfW1QE3L6KG2YWEZMi8CvZ5Yorz85Rrc	oracle	2023-10-31 23:00:00.0+00	2023-10-31 23:00:00.0+00
13JqKYh8eUkRE2aQcTckBmBo82AhrHmE219ZBbraYTKKgGEh2QF	oracle	2023-10-31 22:00:00.0+00	2023-10-31 22:00:00.0+00
141jjqmhrBAE3qtR3ggFwAVcd5StGDRpP6dRRFgsXC7rV32Kngt	administrator	2023-10-31 21:00:00.0+00	2023-10-31 21:00:00.0+00
\.

ALTER TABLE ONLY public.admin_keys
    ADD CONSTRAINT admin_keys_pubkey_key UNIQUE (pubkey);

CREATE TRIGGER set_updated_at BEFORE UPDATE ON public.admin_keys FOR EACH ROW WHEN ((old.* IS DISTINCT FROM new.*)) EXECUTE FUNCTION public.set_updated_at();

-- PostgreSQL database dump complete
