-- ============================================
-- EmuWorld — Bucket Storage pour images du chat
-- Exécuter dans Supabase SQL Editor
-- ============================================

-- Créer le bucket (public pour que les URLs soient accessibles)
INSERT INTO storage.buckets (id, name, public)
VALUES ('chat-images', 'chat-images', true)
ON CONFLICT (id) DO NOTHING;

-- RLS: un user authentifié peut uploader dans son dossier chat/{user_id}/
CREATE POLICY "Users can upload chat images"
  ON storage.objects FOR INSERT
  WITH CHECK (
    bucket_id = 'chat-images'
    AND auth.uid()::text = (storage.foldername(name))[2]
  );

-- RLS: tout le monde peut lire les images (public bucket)
CREATE POLICY "Anyone can view chat images"
  ON storage.objects FOR SELECT
  USING (bucket_id = 'chat-images');

-- RLS: un user peut supprimer ses propres images
CREATE POLICY "Users can delete own chat images"
  ON storage.objects FOR DELETE
  USING (
    bucket_id = 'chat-images'
    AND auth.uid()::text = (storage.foldername(name))[2]
  );
