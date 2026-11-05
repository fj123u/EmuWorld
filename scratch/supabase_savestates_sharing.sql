-- ============================================
-- EmuWorld — Partage de save states entre amis
-- Exécuter dans Supabase SQL Editor (rejouable)
-- ============================================
-- Un save state partagé = un fichier dans le bucket privé `savestates`
-- (<sender_id>/<uuid>.state) + une ligne dans `shared_savestates`.
-- Seuls l'expéditeur et le destinataire peuvent lire le fichier, et on ne
-- peut envoyer qu'à un ami accepté.

-- 1. Table
CREATE TABLE IF NOT EXISTS shared_savestates (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  sender_id uuid NOT NULL DEFAULT auth.uid() REFERENCES auth.users(id) ON DELETE CASCADE,
  recipient_id uuid NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
  console text NOT NULL CHECK (char_length(console) <= 100),
  game_name text NOT NULL CHECK (char_length(game_name) <= 300),
  core text CHECK (core IS NULL OR char_length(core) <= 100),
  slot integer CHECK (slot IS NULL OR slot BETWEEN 0 AND 999),
  label text CHECK (label IS NULL OR char_length(label) <= 80),
  storage_path text NOT NULL UNIQUE,
  size_bytes bigint NOT NULL CHECK (size_bytes > 0 AND size_bytes <= 33554432),
  thumbnail text CHECK (thumbnail IS NULL OR (thumbnail LIKE 'data:image/jpeg;base64,%' AND char_length(thumbnail) <= 60000)),
  created_at timestamptz NOT NULL DEFAULT now(),
  CHECK (sender_id <> recipient_id),
  CHECK (storage_path LIKE sender_id::text || '/%')
);

CREATE INDEX IF NOT EXISTS idx_shared_savestates_recipient ON shared_savestates(recipient_id, created_at DESC);

ALTER TABLE shared_savestates ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Sender and recipient can view shared states" ON shared_savestates;
CREATE POLICY "Sender and recipient can view shared states"
  ON shared_savestates FOR SELECT
  USING (auth.uid() = sender_id OR auth.uid() = recipient_id);

DROP POLICY IF EXISTS "Users share states with accepted friends" ON shared_savestates;
CREATE POLICY "Users share states with accepted friends"
  ON shared_savestates FOR INSERT
  WITH CHECK (
    auth.uid() = sender_id
    AND EXISTS (
      SELECT 1 FROM friendships f
      WHERE f.status = 'accepted'
        AND ((f.requester_id = sender_id AND f.addressee_id = recipient_id)
          OR (f.requester_id = recipient_id AND f.addressee_id = sender_id))
    )
  );

-- Pas d'UPDATE : un partage ne se modifie pas.
DROP POLICY IF EXISTS "Sender or recipient can delete shared states" ON shared_savestates;
CREATE POLICY "Sender or recipient can delete shared states"
  ON shared_savestates FOR DELETE
  USING (auth.uid() = sender_id OR auth.uid() = recipient_id);

-- 2. Bucket privé (32 Mo max par fichier)
INSERT INTO storage.buckets (id, name, public, file_size_limit)
VALUES ('savestates', 'savestates', false, 33554432)
ON CONFLICT (id) DO UPDATE SET public = false, file_size_limit = 33554432;

-- Upload uniquement dans son propre dossier <user_id>/
DROP POLICY IF EXISTS "Users upload save states to own folder" ON storage.objects;
CREATE POLICY "Users upload save states to own folder"
  ON storage.objects FOR INSERT
  WITH CHECK (
    bucket_id = 'savestates'
    AND auth.uid()::text = (storage.foldername(name))[1]
  );

-- Lecture : l'expéditeur, ou le destinataire d'un partage qui pointe vers ce fichier
DROP POLICY IF EXISTS "Sender and recipient read shared save states" ON storage.objects;
CREATE POLICY "Sender and recipient read shared save states"
  ON storage.objects FOR SELECT
  USING (
    bucket_id = 'savestates'
    AND (
      auth.uid()::text = (storage.foldername(name))[1]
      OR EXISTS (
        SELECT 1 FROM public.shared_savestates s
        WHERE s.storage_path = name AND s.recipient_id = auth.uid()
      )
    )
  );

-- Suppression : l'expéditeur, ou le destinataire (après import / refus)
DROP POLICY IF EXISTS "Sender and recipient delete shared save states" ON storage.objects;
CREATE POLICY "Sender and recipient delete shared save states"
  ON storage.objects FOR DELETE
  USING (
    bucket_id = 'savestates'
    AND (
      auth.uid()::text = (storage.foldername(name))[1]
      OR EXISTS (
        SELECT 1 FROM public.shared_savestates s
        WHERE s.storage_path = name AND s.recipient_id = auth.uid()
      )
    )
  );
