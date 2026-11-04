-- ============================================================
-- EmuWorld — Per-month playtime columns (Wrap mensuel)
-- Run this in the Supabase SQL editor. Safe to re-run.
--
-- Mirrors the fields added to src-tauri/src/playtime.rs::GameEntry:
--   monthly          { "YYYY-MM": { seconds, launches, longest_session } }
--   active_days      [ "YYYY-MM-DD", ... ]  (sorted, capped at 400 by the app)
--   longest_session  longest single session ever, in seconds
--
-- Why it matters: on every sign-in (so on every app start with a saved
-- session) the app replaces playtime.json with the cloud rows. Without these
-- columns the per-month counters only live locally; the app keeps the local
-- copy while the columns are missing, but they won't follow the user to
-- another PC until this migration has run.
--
-- Older app versions don't send these columns: their upserts leave them
-- untouched, so running this before everyone has updated is safe.
-- ============================================================

alter table public.playtime_games
  add column if not exists monthly jsonb not null default '{}'::jsonb,
  add column if not exists active_days jsonb not null default '[]'::jsonb,
  add column if not exists longest_session bigint not null default 0;

-- Same sanity bounds as `seconds_sane` (v2.5.0 security fixes): reject
-- absurd values a tampered client could push to the public leaderboards.
alter table public.playtime_games drop constraint if exists longest_session_sane;
alter table public.playtime_games add constraint longest_session_sane
  check (longest_session >= 0 and longest_session <= 3600000);

alter table public.playtime_games drop constraint if exists monthly_shape_sane;
alter table public.playtime_games add constraint monthly_shape_sane
  check (jsonb_typeof(monthly) = 'object' and jsonb_typeof(active_days) = 'array'
         and jsonb_array_length(active_days) <= 400);

-- Make PostgREST see the new columns right away (otherwise the app gets
-- PGRST204 "column not found" until the schema cache refreshes).
notify pgrst, 'reload schema';

-- ============================================================
-- Verify:
--   select console, name, monthly, active_days, longest_session
--   from public.playtime_games limit 5;
-- ============================================================
