-- Weekly challenges
CREATE TABLE IF NOT EXISTS weekly_challenges (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  title text NOT NULL,
  description text NOT NULL,
  game_name text,
  game_console text,
  goal_type text NOT NULL CHECK (goal_type IN ('playtime', 'launches', 'speedrun', 'any_playtime')),
  goal_value integer NOT NULL,
  badge_icon text NOT NULL DEFAULT '🏆',
  start_date date NOT NULL,
  end_date date NOT NULL,
  created_at timestamptz DEFAULT now() NOT NULL
);

-- Challenge participants / progress
CREATE TABLE IF NOT EXISTS challenge_participants (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id uuid NOT NULL,
  challenge_id uuid REFERENCES weekly_challenges(id) ON DELETE CASCADE NOT NULL,
  progress integer DEFAULT 0 NOT NULL,
  completed boolean DEFAULT false NOT NULL,
  completed_at timestamptz,
  joined_at timestamptz DEFAULT now() NOT NULL,
  UNIQUE (user_id, challenge_id)
);

CREATE INDEX idx_challenge_participants_challenge ON challenge_participants(challenge_id);
CREATE INDEX idx_challenge_participants_user ON challenge_participants(user_id);

-- RLS
ALTER TABLE weekly_challenges ENABLE ROW LEVEL SECURITY;
ALTER TABLE challenge_participants ENABLE ROW LEVEL SECURITY;

-- Challenges are public to read
CREATE POLICY "Challenges are public" ON weekly_challenges FOR SELECT USING (true);

-- Participants: anyone can read, users manage their own
CREATE POLICY "Participants are public" ON challenge_participants FOR SELECT USING (true);
CREATE POLICY "Users join challenges" ON challenge_participants FOR INSERT WITH CHECK (auth.uid() = user_id);
CREATE POLICY "Users update own progress" ON challenge_participants FOR UPDATE USING (auth.uid() = user_id);
CREATE POLICY "Users leave challenges" ON challenge_participants FOR DELETE USING (auth.uid() = user_id);

-- Seed: first batch of challenges (rotate weekly)
INSERT INTO weekly_challenges (title, description, game_name, game_console, goal_type, goal_value, badge_icon, start_date, end_date) VALUES
('Marathon Mario', 'Joue 2 heures à un jeu Mario cette semaine', NULL, NULL, 'any_playtime', 7200, '🍄', '2026-05-26', '2026-06-01'),
('Découvreur', 'Lance 5 jeux différents cette semaine', NULL, NULL, 'launches', 5, '🔍', '2026-06-02', '2026-06-08'),
('Endurance', 'Accumule 5 heures de jeu total cette semaine', NULL, NULL, 'any_playtime', 18000, '⏱️', '2026-06-09', '2026-06-15'),
('Rétro Hunter', 'Joue 1 heure à un jeu NES ou SNES', NULL, NULL, 'any_playtime', 3600, '👾', '2026-06-16', '2026-06-22');
