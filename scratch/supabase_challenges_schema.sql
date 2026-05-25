-- Challenge participants / progress
-- Challenges are auto-generated client-side from a rotating pool.
-- This table only stores participation + progress per user.
-- challenge_id is a string like "2026-W21_marathon_mario"

CREATE TABLE IF NOT EXISTS challenge_participants (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id uuid NOT NULL,
  challenge_id text NOT NULL,
  progress integer DEFAULT 0 NOT NULL,
  completed boolean DEFAULT false NOT NULL,
  completed_at timestamptz,
  joined_at timestamptz DEFAULT now() NOT NULL,
  UNIQUE (user_id, challenge_id)
);

CREATE INDEX idx_challenge_participants_challenge ON challenge_participants(challenge_id);
CREATE INDEX idx_challenge_participants_user ON challenge_participants(user_id);

-- RLS
ALTER TABLE challenge_participants ENABLE ROW LEVEL SECURITY;

-- Anyone can read (for leaderboards)
CREATE POLICY "Participants are public" ON challenge_participants FOR SELECT USING (true);
CREATE POLICY "Users join challenges" ON challenge_participants FOR INSERT WITH CHECK (auth.uid() = user_id);
CREATE POLICY "Users update own progress" ON challenge_participants FOR UPDATE USING (auth.uid() = user_id);
CREATE POLICY "Users leave challenges" ON challenge_participants FOR DELETE USING (auth.uid() = user_id);
