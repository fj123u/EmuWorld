-- ============================================
-- EmuWorld — Chat des lobbys multijoueur
-- Exécuter dans Supabase SQL Editor (rejouable sans risque)
-- Dépend de scratch/supabase_lobbies_rls_v2.sql (fonctions private.*)
-- ============================================
-- • seuls l'hôte et les membres (ou invités) d'un lobby lisent ses messages ;
-- • on n'écrit qu'en son nom, et seulement dans un lobby dont on fait partie ;
-- • pas de modification ni de suppression : les messages partent avec le
--   lobby (ON DELETE CASCADE quand l'hôte le dissout) ;
-- • 1 à 500 caractères (sans compter les espaces autour).

CREATE TABLE IF NOT EXISTS public.lobby_messages (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  lobby_id uuid NOT NULL REFERENCES public.lobbies(id) ON DELETE CASCADE,
  user_id uuid NOT NULL DEFAULT auth.uid() REFERENCES auth.users(id) ON DELETE CASCADE,
  content text NOT NULL CHECK (char_length(btrim(content)) BETWEEN 1 AND 500),
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lobby_messages_lobby ON public.lobby_messages(lobby_id, created_at);
CREATE INDEX IF NOT EXISTS idx_lobby_messages_user ON public.lobby_messages(user_id);

ALTER TABLE public.lobby_messages ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Lobby people read its chat" ON public.lobby_messages;
CREATE POLICY "Lobby people read its chat" ON public.lobby_messages
  FOR SELECT TO authenticated USING (
    (SELECT private.is_lobby_member(lobby_id, auth.uid()))
    OR (SELECT private.is_lobby_host(lobby_id, auth.uid()))
  );

DROP POLICY IF EXISTS "Lobby people write in its chat" ON public.lobby_messages;
CREATE POLICY "Lobby people write in its chat" ON public.lobby_messages
  FOR INSERT TO authenticated WITH CHECK (
    user_id = (SELECT auth.uid())
    AND (
      (SELECT private.is_lobby_member(lobby_id, auth.uid()))
      OR (SELECT private.is_lobby_host(lobby_id, auth.uid()))
    )
  );

-- Lecture et envoi seulement ; rien pour les visiteurs sans compte.
REVOKE ALL ON public.lobby_messages FROM anon, authenticated;
GRANT SELECT, INSERT ON public.lobby_messages TO authenticated;

-- Messages en direct (Supabase Realtime applique les mêmes règles de lecture).
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_publication_tables WHERE pubname = 'supabase_realtime' AND schemaname = 'public' AND tablename = 'lobby_messages') THEN
    ALTER PUBLICATION supabase_realtime ADD TABLE public.lobby_messages;
  END IF;
END $$;

-- Vérification : 2 règles (SELECT, INSERT).
SELECT policyname, cmd FROM pg_policies WHERE schemaname = 'public' AND tablename = 'lobby_messages';
