-- Multiplayer lobbies
CREATE TABLE IF NOT EXISTS lobbies (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  host_id uuid NOT NULL,
  game_name text NOT NULL,
  game_console text NOT NULL,
  status text NOT NULL DEFAULT 'waiting' CHECK (status IN ('waiting', 'ready', 'playing', 'closed')),
  max_players integer DEFAULT 2 NOT NULL,
  netplay_code text,
  created_at timestamptz DEFAULT now() NOT NULL,
  updated_at timestamptz DEFAULT now() NOT NULL
);

CREATE TABLE IF NOT EXISTS lobby_members (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  lobby_id uuid REFERENCES lobbies(id) ON DELETE CASCADE NOT NULL,
  user_id uuid NOT NULL,
  is_ready boolean DEFAULT false NOT NULL,
  joined_at timestamptz DEFAULT now() NOT NULL,
  UNIQUE (lobby_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_lobbies_host ON lobbies(host_id);
CREATE INDEX IF NOT EXISTS idx_lobbies_status ON lobbies(status);
CREATE INDEX IF NOT EXISTS idx_lobby_members_lobby ON lobby_members(lobby_id);
CREATE INDEX IF NOT EXISTS idx_lobby_members_user ON lobby_members(user_id);

-- RLS
ALTER TABLE lobbies ENABLE ROW LEVEL SECURITY;
ALTER TABLE lobby_members ENABLE ROW LEVEL SECURITY;

-- Lobbies visible by friends only (simplified: public for now)
CREATE POLICY "Lobbies are public" ON lobbies FOR SELECT USING (true);
CREATE POLICY "Users create lobbies" ON lobbies FOR INSERT WITH CHECK (auth.uid() = host_id);
CREATE POLICY "Host updates lobby" ON lobbies FOR UPDATE USING (auth.uid() = host_id);
CREATE POLICY "Host deletes lobby" ON lobbies FOR DELETE USING (auth.uid() = host_id);

-- Members
CREATE POLICY "Members are public" ON lobby_members FOR SELECT USING (true);
CREATE POLICY "Users join lobbies" ON lobby_members FOR INSERT WITH CHECK (auth.uid() = user_id);
CREATE POLICY "Users leave lobbies" ON lobby_members FOR DELETE USING (auth.uid() = user_id);
CREATE POLICY "Users update own ready" ON lobby_members FOR UPDATE USING (auth.uid() = user_id);

-- Enable realtime
ALTER PUBLICATION supabase_realtime ADD TABLE lobbies;
ALTER PUBLICATION supabase_realtime ADD TABLE lobby_members;
