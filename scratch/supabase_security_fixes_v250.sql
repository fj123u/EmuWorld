-- ============================================
-- EmuWorld — Security fixes from audit v2.5.0
-- P27: Restrict user_achievements, activity_feed, lobbies, lobby_members
-- P28: Messages require friendship
-- P29: Playtime seconds cap
-- Run in Supabase SQL Editor
-- ============================================

-- =====================
-- P27 — Restrict SELECT on exposed tables to authenticated users only
-- =====================

-- user_achievements: need auth to read (for rarity %)
DROP POLICY IF EXISTS "Anyone can read achievements" ON user_achievements;
CREATE POLICY "Authenticated can read achievements" ON user_achievements
  FOR SELECT USING (auth.uid() IS NOT NULL);

-- activity_feed: only friends can see (or own)
DROP POLICY IF EXISTS "Anyone can view activity" ON activity_feed;
CREATE POLICY "Authenticated can view activity" ON activity_feed
  FOR SELECT USING (
    auth.uid() IS NOT NULL
    AND (
      auth.uid() = user_id
      OR EXISTS (
        SELECT 1 FROM friendships
        WHERE (user_id = auth.uid() AND friend_id = activity_feed.user_id AND status = 'accepted')
           OR (user_id = activity_feed.user_id AND friend_id = auth.uid() AND status = 'accepted')
      )
    )
  );

-- lobbies: only authenticated users
DROP POLICY IF EXISTS "Lobbies are public" ON lobbies;
CREATE POLICY "Authenticated can view lobbies" ON lobbies
  FOR SELECT USING (auth.uid() IS NOT NULL);

-- lobby_members: only authenticated users
DROP POLICY IF EXISTS "Members are public" ON lobby_members;
CREATE POLICY "Authenticated can view members" ON lobby_members
  FOR SELECT USING (auth.uid() IS NOT NULL);

-- =====================
-- P28 — Messages require friendship between sender and receiver
-- =====================

DROP POLICY IF EXISTS "Users can send messages" ON messages;
CREATE POLICY "Users can send messages to friends" ON messages
  FOR INSERT WITH CHECK (
    auth.uid() = sender_id
    AND EXISTS (
      SELECT 1 FROM friendships
      WHERE status = 'accepted'
        AND (
          (user_id = auth.uid() AND friend_id = receiver_id)
          OR (user_id = receiver_id AND friend_id = auth.uid())
        )
    )
  );

-- =====================
-- P29 — Cap playtime seconds to a sane maximum (1,000,000s ≈ 277h per game)
-- =====================

ALTER TABLE playtime_games DROP CONSTRAINT IF EXISTS seconds_sane;
ALTER TABLE playtime_games ADD CONSTRAINT seconds_sane CHECK (seconds >= 0 AND seconds <= 3600000);
-- 3,600,000s = 1000h per game — generous enough for any real player
