-- EmuWorld — challenges v2 (steps 4 and 6 of scratch/defis-refonte.md)
-- Written 2026-11-22. Applied to the EmuWorld project on 2026-11-22 with the user's OK
-- (migration challenges_v2).
--
-- 1. challenge_progress: each player's progress on the COMMON challenges of a
--    period (personal ones are not ranked, they are never sent). Visible to the
--    player and their accepted friends only (the old challenge_participants was
--    public). Guards: progress never goes down, never above the goal, never
--    more than the time since the period started (+ 1 day of time-zone slack).
-- 2. versus_challenges (status values already allowed: pending, active,
--    completed, declined): the rules of a versus as JSON, start / end set when
--    the opponent accepts, each player writes only their own progress, and
--    only close_versus() (after the end) sets the winner.
-- challenge_participants is left as it is: older app versions still use it.

-- ---- 1. challenge_progress ------------------------------------------------------

create table if not exists public.challenge_progress (
  user_id uuid not null references auth.users (id) on delete cascade,
  -- J-2026-11-22 / S-2026-W47 / M-2026-11 (src/challenges.ts periodOf)
  period_key text not null check (period_key ~ '^(J-\d{4}-\d{2}-\d{2}|S-\d{4}-W\d{2}|M-\d{4}-\d{2})$'),
  -- common challenges only: week_… / month_…
  challenge_id text not null check (challenge_id ~ '^(week|month)_[a-z0-9_]{1,40}$'),
  progress integer not null default 0 check (progress >= 0),
  goal integer not null check (goal > 0 and goal <= 10000000),
  completed_at timestamptz,
  updated_at timestamptz not null default now(),
  primary key (user_id, period_key, challenge_id),
  check (progress <= goal)
);

alter table public.challenge_progress enable row level security;

-- Start of a period key, in UTC (the client uses local time: the guard below
-- allows one day of slack for that).
create or replace function public.challenge_period_start(key text)
returns timestamptz
language sql
stable
set search_path = ''
as $$
  select case left(key, 1)
    when 'J' then to_date(substr(key, 3), 'YYYY-MM-DD')::timestamptz
    when 'S' then to_date(substr(key, 3, 4) || substr(key, 9, 2), 'IYYYIW')::timestamptz
    when 'M' then to_date(substr(key, 3) || '-01', 'YYYY-MM-DD')::timestamptz
  end
$$;

create or replace function public.challenge_progress_guard()
returns trigger
language plpgsql
set search_path = ''
as $$
declare
  elapsed numeric := extract(epoch from (now() - public.challenge_period_start(new.period_key))) + 86400;
begin
  if tg_op = 'UPDATE' then
    if new.progress < old.progress then
      new.progress := old.progress; -- never backwards (an older client, another PC)
    end if;
    new.goal := old.goal;
    if old.completed_at is not null then
      new.completed_at := old.completed_at;
    end if;
  end if;
  if elapsed < 0 or new.progress > elapsed then
    raise exception 'challenge progress % not possible for %', new.progress, new.period_key;
  end if;
  if new.progress >= new.goal and new.completed_at is null then
    new.completed_at := now();
  end if;
  new.updated_at := now();
  return new;
end;
$$;

drop trigger if exists challenge_progress_guard on public.challenge_progress;
create trigger challenge_progress_guard
  before insert or update on public.challenge_progress
  for each row execute function public.challenge_progress_guard();

drop policy if exists "Progress visible to self and friends" on public.challenge_progress;
create policy "Progress visible to self and friends" on public.challenge_progress
  for select to authenticated
  using (
    (select auth.uid()) = user_id
    or exists (
      select 1 from public.friendships f
      where f.status = 'accepted'
        and ((f.requester_id = (select auth.uid()) and f.addressee_id = challenge_progress.user_id)
          or (f.addressee_id = (select auth.uid()) and f.requester_id = challenge_progress.user_id))
    )
  );

drop policy if exists "Users insert own progress" on public.challenge_progress;
create policy "Users insert own progress" on public.challenge_progress
  for insert to authenticated
  with check ((select auth.uid()) = user_id);

drop policy if exists "Users update own progress" on public.challenge_progress;
create policy "Users update own progress" on public.challenge_progress
  for update to authenticated
  using ((select auth.uid()) = user_id)
  with check ((select auth.uid()) = user_id);

create index if not exists challenge_progress_period on public.challenge_progress (period_key, challenge_id);

-- ---- 2. versus_challenges ---------------------------------------------------------

alter table public.versus_challenges
  add column if not exists rules jsonb,           -- {"kind": "time"|"distinct"|"days"|"sessions", "console"?, "game"?}
  add column if not exists started_at timestamptz,
  add column if not exists finished_at timestamptz;

-- What each player may change. close_versus() sets emuworld.closing to pass.
create or replace function public.versus_guard()
returns trigger
language plpgsql
set search_path = ''
as $$
declare
  me uuid := (select auth.uid());
  closing boolean := coalesce(current_setting('emuworld.closing', true), '') = 'on';
begin
  -- close_versus(), or a change made from the dashboard (no signed-in user:
  -- the app's requests always have one, RLS turns the others away).
  if closing or me is null then
    return new;
  end if;
  -- Fixed once created.
  new.challenger_id := old.challenger_id;
  new.opponent_id := old.opponent_id;
  new.challenge_type := old.challenge_type;
  new.rules := old.rules;
  new.duration_days := old.duration_days;
  new.created_at := old.created_at;
  -- Only close_versus() decides the winner.
  new.winner_id := old.winner_id;
  new.finished_at := old.finished_at;
  if new.status = 'completed' and old.status <> 'completed' then
    raise exception 'a versus is closed by close_versus()';
  end if;
  -- Accepting / declining: the opponent, while pending. Accepting starts it.
  if new.status is distinct from old.status then
    if not (old.status = 'pending' and me = old.opponent_id and new.status in ('active', 'declined')) then
      raise exception 'status change not allowed';
    end if;
    if new.status = 'active' then
      new.started_at := now();
      new.ends_at := now() + make_interval(days => greatest(1, least(old.duration_days, 31)));
    end if;
  else
    new.started_at := old.started_at;
    new.ends_at := old.ends_at;
  end if;
  -- Each player writes their own progress only, never backwards, and only
  -- while the versus runs.
  if me = old.challenger_id then
    new.opponent_progress := old.opponent_progress;
    new.challenger_progress := greatest(old.challenger_progress, new.challenger_progress);
  elsif me = old.opponent_id then
    new.challenger_progress := old.challenger_progress;
    new.opponent_progress := greatest(old.opponent_progress, new.opponent_progress);
  else
    raise exception 'not a participant';
  end if;
  if old.status <> 'active' then
    new.challenger_progress := old.challenger_progress;
    new.opponent_progress := old.opponent_progress;
  elsif new.started_at is not null and greatest(new.challenger_progress, new.opponent_progress)
      > extract(epoch from (now() - new.started_at)) + 60 then
    raise exception 'versus progress not possible';
  end if;
  return new;
end;
$$;

drop trigger if exists versus_guard on public.versus_challenges;
create trigger versus_guard
  before update on public.versus_challenges
  for each row execute function public.versus_guard();

-- Ends a versus whose time is up and names the winner (null: a draw).
create or replace function public.close_versus(versus_id uuid)
returns public.versus_challenges
language plpgsql
security definer
set search_path = ''
as $$
declare
  v public.versus_challenges;
begin
  select * into v from public.versus_challenges where id = versus_id for update;
  if not found or (select auth.uid()) not in (v.challenger_id, v.opponent_id) then
    raise exception 'versus not found';
  end if;
  if v.status <> 'active' or v.ends_at is null or now() < v.ends_at then
    return v;
  end if;
  perform set_config('emuworld.closing', 'on', true);
  update public.versus_challenges
     set status = 'completed',
         finished_at = now(),
         winner_id = case
           when v.challenger_progress > v.opponent_progress then v.challenger_id
           when v.opponent_progress > v.challenger_progress then v.opponent_id
         end
   where id = versus_id
  returning * into v;
  perform set_config('emuworld.closing', 'off', true);
  return v;
end;
$$;

revoke all on function public.close_versus(uuid) from public, anon;
grant execute on function public.close_versus(uuid) to authenticated;
