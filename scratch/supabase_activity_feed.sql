-- ============================================
-- EmuWorld — Activity Feed
-- Exécuter dans Supabase SQL Editor
-- ============================================

CREATE TABLE IF NOT EXISTS activity_feed (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  event_type text NOT NULL CHECK (event_type IN ('game_started', 'achievement_unlocked', 'game_added', 'game_completed')),
  game_name text,
  console text,
  details text,
  created_at timestamptz DEFAULT now()
);

CREATE INDEX idx_activity_feed_user ON activity_feed(user_id, created_at DESC);
CREATE INDEX idx_activity_feed_created ON activity_feed(created_at DESC);

ALTER TABLE activity_feed ENABLE ROW LEVEL SECURITY;

-- Tout le monde peut voir l'activité (filtrage côté app par amis)
CREATE POLICY "Anyone can view activity"
  ON activity_feed FOR SELECT
  USING (true);

-- Chacun ne peut insérer que ses propres événements
CREATE POLICY "Users can insert own activity"
  ON activity_feed FOR INSERT
  WITH CHECK (auth.uid() = user_id);

-- Chacun peut supprimer ses propres événements
CREATE POLICY "Users can delete own activity"
  ON activity_feed FOR DELETE
  USING (auth.uid() = user_id);

-- Realtime
ALTER PUBLICATION supabase_realtime ADD TABLE activity_feed;
