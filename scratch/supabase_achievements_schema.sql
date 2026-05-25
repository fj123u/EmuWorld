-- Table: user_achievements
-- Stores which achievements each user has unlocked + when.
-- The app upserts rows on unlock; the view computes rarity stats.

CREATE TABLE IF NOT EXISTS user_achievements (
  user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  achievement_id TEXT NOT NULL,
  unlocked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, achievement_id)
);

-- RLS: users can read all (for rarity %), but only insert/update their own
ALTER TABLE user_achievements ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Anyone can read achievements" ON user_achievements
  FOR SELECT USING (true);

CREATE POLICY "Users can insert their own" ON user_achievements
  FOR INSERT WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update their own" ON user_achievements
  FOR UPDATE USING (auth.uid() = user_id);

-- View: achievement_rarity
-- Computes for each achievement_id: total unlocks, percentage of all users
CREATE OR REPLACE VIEW achievement_rarity
WITH (security_invoker = true) AS
SELECT
  achievement_id,
  COUNT(DISTINCT user_id) AS unlock_count,
  ROUND(
    COUNT(DISTINCT user_id)::numeric / GREATEST((SELECT COUNT(*) FROM profiles), 1) * 100,
    1
  ) AS unlock_percent
FROM user_achievements
GROUP BY achievement_id;

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_user_achievements_user ON user_achievements(user_id);
CREATE INDEX IF NOT EXISTS idx_user_achievements_achievement ON user_achievements(achievement_id);
