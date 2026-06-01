-- Community themes marketplace
CREATE TABLE IF NOT EXISTS community_themes (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id uuid NOT NULL,
  name text NOT NULL,
  description text DEFAULT '',
  base_theme text NOT NULL DEFAULT 'default',
  accent_hue integer,
  custom_css jsonb DEFAULT '{}',
  downloads integer DEFAULT 0 NOT NULL,
  created_at timestamptz DEFAULT now() NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_community_themes_downloads ON community_themes(downloads DESC);
CREATE INDEX IF NOT EXISTS idx_community_themes_user ON community_themes(user_id);

-- RLS
ALTER TABLE community_themes ENABLE ROW LEVEL SECURITY;
CREATE POLICY "Themes are public" ON community_themes FOR SELECT USING (true);
CREATE POLICY "Users create themes" ON community_themes FOR INSERT WITH CHECK (auth.uid() = user_id);
CREATE POLICY "Users delete own themes" ON community_themes FOR DELETE USING (auth.uid() = user_id);

-- Increment download counter function
CREATE OR REPLACE FUNCTION increment_theme_downloads(theme_id uuid)
RETURNS void AS $$
BEGIN
  UPDATE community_themes SET downloads = downloads + 1 WHERE id = theme_id;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
