-- ============================================
-- EmuWorld — Correctifs sécurité v2.6.0 (audit du 06/11/2026)
-- Appliqué via le connecteur Supabase ; rejouable sans risque.
-- ============================================

-- 1. friendships : celui qui ENVOIE une demande pouvait la passer lui-même en
--    « accepted » (la règle UPDATE autorisait aussi requester_id), puis
--    écrire, inviter, partager des states et lire le fil d'activité de la
--    personne sans son accord. Seul le destinataire répond désormais, et il
--    ne peut changer que le statut.
DROP POLICY IF EXISTS "Users can update friendships addressed to them" ON public.friendships;
DROP POLICY IF EXISTS "Addressee answers a friend request" ON public.friendships;
CREATE POLICY "Addressee answers a friend request" ON public.friendships
  FOR UPDATE TO authenticated
  USING (auth.uid() = addressee_id)
  WITH CHECK (auth.uid() = addressee_id AND status IN ('accepted', 'blocked'));
REVOKE UPDATE ON public.friendships FROM authenticated, anon;
GRANT UPDATE (status) ON public.friendships TO authenticated;

-- 2. messages : « marquer comme lu » laissait le destinataire réécrire tout
--    le message (contenu, expéditeur affiché…). Seul read_at change.
REVOKE UPDATE ON public.messages FROM authenticated, anon;
GRANT UPDATE (read_at) ON public.messages TO authenticated;
DROP POLICY IF EXISTS "Users can send messages to friends" ON public.messages; -- doublon de "...to friends only"

-- 3. presence : une ancienne règle (USING true) montrait à n'importe qui sur
--    Internet qui est en ligne et à quoi il joue. Réservé aux comptes connectés.
DROP POLICY IF EXISTS "Anyone can view presence" ON public.presence;

-- 4. Fonctions : pas d'appel anonyme au compteur de téléchargements de
--    thèmes, et search_path figé (lint Supabase 0011).
REVOKE EXECUTE ON FUNCTION public.increment_theme_downloads(uuid) FROM PUBLIC, anon;
GRANT EXECUTE ON FUNCTION public.increment_theme_downloads(uuid) TO authenticated;
ALTER FUNCTION public.increment_theme_downloads(uuid) SET search_path = public;
ALTER FUNCTION public.set_updated_at() SET search_path = public;
ALTER FUNCTION public.update_updated_at() SET search_path = public;

-- 5. Doublons (même effet, noms différents) : une règle par action.
DROP POLICY IF EXISTS "Public profiles are viewable by everyone." ON public.profiles;
DROP POLICY IF EXISTS "profiles_owner_read" ON public.profiles;      -- couvert par "viewable by everyone"
DROP POLICY IF EXISTS "profiles_public_read" ON public.profiles;     -- idem
DROP POLICY IF EXISTS "Users can insert their own profile" ON public.profiles;
DROP POLICY IF EXISTS "Users can insert their own profile." ON public.profiles;
DROP POLICY IF EXISTS "Users can update own profile." ON public.profiles;
DROP POLICY IF EXISTS "Users can update their own profile" ON public.profiles;
-- restent : "Profiles are viewable by everyone", profiles_owner_insert, profiles_owner_update
DROP POLICY IF EXISTS "Achievements visible by authenticated users" ON public.user_achievements; -- = "Authenticated can read achievements"
DROP POLICY IF EXISTS "Avatar Upload" ON storage.objects;            -- = "Avatar User Upload"
