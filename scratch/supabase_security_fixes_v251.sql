-- ============================================
-- EmuWorld — Security fixes from audit v2.5.1
-- C2/H2: Restrict guides/reviews to authenticated + moderation
-- H1: Challenge progress constraint
-- H4: Playtime reads require auth
-- M1: Friend request rate limit
-- Run in Supabase SQL Editor
-- ============================================

-- =====================
-- C2 + H2 — Guides: add moderation + restrict to authenticated
-- =====================

-- Add approved column for moderation (defaults to true for now to not break existing)
ALTER TABLE game_guides ADD COLUMN IF NOT EXISTS approved boolean DEFAULT true;

-- Restrict reads to authenticated users only
DROP POLICY IF EXISTS "Guides are public" ON game_guides;
CREATE POLICY "Authenticated read guides" ON game_guides
  FOR SELECT USING (auth.uid() IS NOT NULL AND (approved = true OR auth.uid() = user_id));

-- Rate limit: max 10 guides per user per day
DROP POLICY IF EXISTS "Users write guides" ON game_guides;
CREATE POLICY "Users write guides rate limited" ON game_guides
  FOR INSERT WITH CHECK (
    auth.uid() = user_id
    AND (SELECT COUNT(*) FROM game_guides WHERE user_id = auth.uid() AND created_at > now() - interval '1 day') < 10
  );

-- =====================
-- H2 — Reviews: restrict to authenticated
-- =====================

DROP POLICY IF EXISTS "Reviews are public" ON game_reviews;
CREATE POLICY "Authenticated read reviews" ON game_reviews
  FOR SELECT USING (auth.uid() IS NOT NULL);

-- =====================
-- H1 — Challenge progress constraint
-- =====================

ALTER TABLE challenge_participants DROP CONSTRAINT IF EXISTS progress_sane;
ALTER TABLE challenge_participants ADD CONSTRAINT progress_sane CHECK (progress >= 0 AND progress <= 10000000);

-- =====================
-- H4 — Playtime public reads require auth
-- =====================

DROP POLICY IF EXISTS "playtime_games_public_read" ON playtime_games;
CREATE POLICY "Authenticated read public playtime" ON playtime_games
  FOR SELECT USING (
    auth.uid() IS NOT NULL
    AND (
      auth.uid() = user_id
      OR EXISTS (SELECT 1 FROM profiles p WHERE p.id = playtime_games.user_id AND p.public_profile = true)
    )
  );

-- =====================
-- M1 — Friend request rate limit (max 20/hour)
-- =====================

CREATE OR REPLACE FUNCTION check_friend_request_rate_limit()
RETURNS TRIGGER AS $$
BEGIN
  IF (SELECT COUNT(*) FROM friendships
      WHERE requester_id = NEW.requester_id
        AND created_at > now() - interval '1 hour') > 20 THEN
    RAISE EXCEPTION 'Rate limit: too many friend requests';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS friend_request_rate_limit ON friendships;
CREATE TRIGGER friend_request_rate_limit
  BEFORE INSERT ON friendships
  FOR EACH ROW EXECUTE FUNCTION check_friend_request_rate_limit();
