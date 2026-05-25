-- Game guides: community-written per game
CREATE TABLE IF NOT EXISTS game_guides (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id uuid NOT NULL,
  game_name text NOT NULL,
  game_console text NOT NULL,
  section text NOT NULL CHECK (section IN ('presentation', 'tips', 'achievements', 'secrets')),
  title text NOT NULL,
  content text NOT NULL,
  upvotes integer DEFAULT 0 NOT NULL,
  created_at timestamptz DEFAULT now() NOT NULL,
  updated_at timestamptz DEFAULT now() NOT NULL
);

-- Upvotes tracking
CREATE TABLE IF NOT EXISTS guide_votes (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id uuid NOT NULL,
  guide_id uuid REFERENCES game_guides(id) ON DELETE CASCADE NOT NULL,
  UNIQUE (user_id, guide_id)
);

CREATE INDEX IF NOT EXISTS idx_game_guides_game ON game_guides(game_name, game_console);
CREATE INDEX IF NOT EXISTS idx_game_guides_section ON game_guides(section);
CREATE INDEX IF NOT EXISTS idx_guide_votes_guide ON guide_votes(guide_id);

-- RLS
ALTER TABLE game_guides ENABLE ROW LEVEL SECURITY;
ALTER TABLE guide_votes ENABLE ROW LEVEL SECURITY;

-- Guides are public to read
CREATE POLICY "Guides are public" ON game_guides FOR SELECT USING (true);
CREATE POLICY "Users write guides" ON game_guides FOR INSERT WITH CHECK (auth.uid() = user_id);
CREATE POLICY "Users edit own guides" ON game_guides FOR UPDATE USING (auth.uid() = user_id);
CREATE POLICY "Users delete own guides" ON game_guides FOR DELETE USING (auth.uid() = user_id);

-- Votes
CREATE POLICY "Votes are public" ON guide_votes FOR SELECT USING (true);
CREATE POLICY "Users vote" ON guide_votes FOR INSERT WITH CHECK (auth.uid() = user_id);
CREATE POLICY "Users unvote" ON guide_votes FOR DELETE USING (auth.uid() = user_id);
