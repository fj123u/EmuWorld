-- ============================================
-- EmuWorld — Chat 1:1 entre amis
-- Exécuter dans Supabase SQL Editor
-- ============================================

-- Table messages
CREATE TABLE IF NOT EXISTS messages (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  sender_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  receiver_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  content text NOT NULL CHECK (char_length(content) <= 2000),
  created_at timestamptz DEFAULT now(),
  read_at timestamptz
);

-- Index pour les requêtes fréquentes
CREATE INDEX idx_messages_conversation ON messages(sender_id, receiver_id, created_at DESC);
CREATE INDEX idx_messages_receiver_unread ON messages(receiver_id, read_at) WHERE read_at IS NULL;

-- RLS
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;

-- Un user peut voir les messages qu'il a envoyés ou reçus
CREATE POLICY "Users can view own messages"
  ON messages FOR SELECT
  USING (auth.uid() = sender_id OR auth.uid() = receiver_id);

-- Un user peut envoyer un message (il est le sender)
CREATE POLICY "Users can send messages"
  ON messages FOR INSERT
  WITH CHECK (auth.uid() = sender_id);

-- Un user peut marquer comme lu les messages qu'il a reçus
CREATE POLICY "Users can mark messages as read"
  ON messages FOR UPDATE
  USING (auth.uid() = receiver_id);

-- Realtime
ALTER PUBLICATION supabase_realtime ADD TABLE messages;
