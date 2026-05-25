-- Reviews / community comments table
CREATE TABLE IF NOT EXISTS game_reviews (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id uuid REFERENCES profiles(id) ON DELETE CASCADE NOT NULL,
  game_name text NOT NULL,
  game_console text NOT NULL,
  rating smallint CHECK (rating >= 1 AND rating <= 5) NOT NULL,
  comment text,
  created_at timestamptz DEFAULT now() NOT NULL,
  updated_at timestamptz DEFAULT now() NOT NULL,
  UNIQUE (user_id, game_name, game_console)
);

-- Index for fast lookup by game
CREATE INDEX idx_game_reviews_game ON game_reviews (game_name, game_console);

-- RLS
ALTER TABLE game_reviews ENABLE ROW LEVEL SECURITY;

-- Anyone can read reviews
CREATE POLICY "Reviews are public" ON game_reviews
  FOR SELECT USING (true);

-- Users can insert their own reviews
CREATE POLICY "Users can insert own reviews" ON game_reviews
  FOR INSERT WITH CHECK (auth.uid() = user_id);

-- Users can update their own reviews
CREATE POLICY "Users can update own reviews" ON game_reviews
  FOR UPDATE USING (auth.uid() = user_id);

-- Users can delete their own reviews
CREATE POLICY "Users can delete own reviews" ON game_reviews
  FOR DELETE USING (auth.uid() = user_id);
