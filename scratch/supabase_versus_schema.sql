-- Versus challenges between friends
CREATE TABLE IF NOT EXISTS versus_challenges (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  challenger_id uuid NOT NULL,
  opponent_id uuid NOT NULL,
  game_name text,
  game_console text,
  challenge_type text NOT NULL CHECK (challenge_type IN ('playtime', 'launches', 'streak')),
  goal_description text NOT NULL,
  duration_days integer NOT NULL DEFAULT 7,
  challenger_progress integer DEFAULT 0 NOT NULL,
  opponent_progress integer DEFAULT 0 NOT NULL,
  winner_id uuid,
  status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'active', 'completed', 'declined')),
  created_at timestamptz DEFAULT now() NOT NULL,
  ends_at timestamptz
);

CREATE INDEX IF NOT EXISTS idx_versus_challenger ON versus_challenges(challenger_id);
CREATE INDEX IF NOT EXISTS idx_versus_opponent ON versus_challenges(opponent_id);
CREATE INDEX IF NOT EXISTS idx_versus_status ON versus_challenges(status);

-- RLS
ALTER TABLE versus_challenges ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Versus visible by participants" ON versus_challenges FOR SELECT USING (auth.uid() = challenger_id OR auth.uid() = opponent_id);
CREATE POLICY "Users create versus" ON versus_challenges FOR INSERT WITH CHECK (auth.uid() = challenger_id);
CREATE POLICY "Participants update versus" ON versus_challenges FOR UPDATE USING (auth.uid() = challenger_id OR auth.uid() = opponent_id);
CREATE POLICY "Challenger deletes versus" ON versus_challenges FOR DELETE USING (auth.uid() = challenger_id);
