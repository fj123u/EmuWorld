-- ============================================================
-- EmuWorld — Cloud playtime sync schema
-- Run this in the Supabase SQL editor.
-- Safe to re-run: every statement uses IF NOT EXISTS / CREATE OR REPLACE.
-- ============================================================

-- 1. profiles.public_profile — lets a user opt-in to being discoverable on the web.
alter table public.profiles
  add column if not exists public_profile boolean not null default false;

-- 2. Per-game aggregates. Mirrors src-tauri/src/playtime.rs::GameEntry.
create table if not exists public.playtime_games (
  user_id uuid not null references auth.users(id) on delete cascade,
  console text not null,
  name text not null,
  seconds bigint not null default 0,
  launches integer not null default 0,
  last_played timestamptz,
  first_played timestamptz,
  favorite boolean not null default false,
  last_emulator_id text,
  updated_at timestamptz not null default now(),
  primary key (user_id, console, name)
);

create index if not exists playtime_games_user_idx
  on public.playtime_games (user_id);

create index if not exists playtime_games_public_seconds_idx
  on public.playtime_games (seconds desc);

-- 3. Per-emulator aggregates (for "top emulator" stat + leaderboards).
create table if not exists public.playtime_emulators (
  user_id uuid not null references auth.users(id) on delete cascade,
  emulator_id text not null,
  seconds bigint not null default 0,
  updated_at timestamptz not null default now(),
  primary key (user_id, emulator_id)
);

create index if not exists playtime_emulators_user_idx
  on public.playtime_emulators (user_id);

-- 4. Trigger: keep updated_at fresh on every UPSERT.
create or replace function public.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

drop trigger if exists playtime_games_touch on public.playtime_games;
create trigger playtime_games_touch
  before update on public.playtime_games
  for each row execute function public.set_updated_at();

drop trigger if exists playtime_emulators_touch on public.playtime_emulators;
create trigger playtime_emulators_touch
  before update on public.playtime_emulators
  for each row execute function public.set_updated_at();

-- 5. Row Level Security.
alter table public.playtime_games enable row level security;
alter table public.playtime_emulators enable row level security;

-- 5a. Owner can read/write their own rows (covers insert / update / delete / select).
drop policy if exists playtime_games_owner_rw on public.playtime_games;
create policy playtime_games_owner_rw
  on public.playtime_games
  for all
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

drop policy if exists playtime_emulators_owner_rw on public.playtime_emulators;
create policy playtime_emulators_owner_rw
  on public.playtime_emulators
  for all
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

-- 5b. Anyone can SELECT rows that belong to a user whose profile is public.
drop policy if exists playtime_games_public_read on public.playtime_games;
create policy playtime_games_public_read
  on public.playtime_games
  for select
  using (
    exists (
      select 1 from public.profiles p
      where p.id = playtime_games.user_id
        and p.public_profile = true
    )
  );

drop policy if exists playtime_emulators_public_read on public.playtime_emulators;
create policy playtime_emulators_public_read
  on public.playtime_emulators
  for select
  using (
    exists (
      select 1 from public.profiles p
      where p.id = playtime_emulators.user_id
        and p.public_profile = true
    )
  );

-- 5c. Public profiles (username + avatar + flag) must be readable by anyone so that
-- the /u/<pseudo> web page can resolve the pseudo and show the avatar without a JWT.
-- Existing "owner can read their own profile" policy is preserved; we only ADD this
-- read-all policy for rows where public_profile = true.
drop policy if exists profiles_public_read on public.profiles;
create policy profiles_public_read
  on public.profiles
  for select
  using (public_profile = true);

-- 6. Helper view used by the web-panel leaderboard (aggregates per public user).
-- Safe to recreate; no data stored here.
create or replace view public.playtime_leaderboard
with (security_invoker = true) as
select
  p.id as user_id,
  p.username,
  p.avatar_url,
  coalesce(sum(g.seconds), 0)::bigint as total_seconds,
  coalesce(sum(g.launches), 0)::integer as total_launches,
  count(distinct (g.console || '::' || g.name))::integer as games_played
from public.profiles p
left join public.playtime_games g on g.user_id = p.id
where p.public_profile = true
group by p.id, p.username, p.avatar_url;

-- Grants so the anon role (used by the unauthenticated web page) can read the view.
grant select on public.playtime_leaderboard to anon, authenticated;

-- ============================================================
-- Done. Verify:
--   select * from public.playtime_games limit 1;
--   select * from public.playtime_leaderboard limit 1;
-- ============================================================
