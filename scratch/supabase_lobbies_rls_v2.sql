-- ============================================
-- EmuWorld — Lobbys multijoueur : règles d'accès v2
-- Exécuter dans Supabase SQL Editor (rejouable sans risque)
-- ============================================
-- Avant : tout compte connecté voyait tous les lobbys, pouvait s'ajouter à
-- n'importe lequel, et un hôte pouvait « inviter » n'importe qui.
-- Après :
--   • un lobby n'est visible que par son hôte et ses membres/invités ;
--   • seul l'hôte ajoute des membres : lui-même, ou un AMI accepté (invitation) ;
--   • un membre ne peut changer que son « prêt » (is_ready), rien d'autre ;
--   • on quitte soi-même, et l'hôte peut retirer les membres de son lobby ;
--   • l'hôte reste l'hôte (pas de transfert via UPDATE) ;
--   • netplay_code limité à 2000 caractères ;
--   • règles réservées aux comptes connectés (sans compte : liste vide).

-- 1. Fonctions d'aide, dans un schéma « private » que l'API n'expose pas.
--    Dans « public », Supabase les rend appelables par tous via /rest/v1/rpc
--    (droits par défaut) : n'importe qui pouvait tester si deux comptes sont
--    amis. SECURITY DEFINER évite la récursion entre les règles des deux
--    tables ; elles ne renvoient qu'un booléen.
CREATE SCHEMA IF NOT EXISTS private;
REVOKE ALL ON SCHEMA private FROM PUBLIC, anon;
GRANT USAGE ON SCHEMA private TO authenticated;

CREATE OR REPLACE FUNCTION private.is_lobby_host(p_lobby uuid, p_user uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public AS $$
  SELECT EXISTS (SELECT 1 FROM public.lobbies WHERE id = p_lobby AND host_id = p_user);
$$;

CREATE OR REPLACE FUNCTION private.is_lobby_member(p_lobby uuid, p_user uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public AS $$
  SELECT EXISTS (SELECT 1 FROM public.lobby_members WHERE lobby_id = p_lobby AND user_id = p_user);
$$;

CREATE OR REPLACE FUNCTION private.are_friends(p_a uuid, p_b uuid)
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public AS $$
  SELECT EXISTS (
    SELECT 1 FROM public.friendships
    WHERE status = 'accepted'
      AND ((requester_id = p_a AND addressee_id = p_b) OR (requester_id = p_b AND addressee_id = p_a))
  );
$$;

REVOKE ALL ON ALL FUNCTIONS IN SCHEMA private FROM PUBLIC, anon;
GRANT EXECUTE ON FUNCTION private.is_lobby_host(uuid, uuid) TO authenticated;
GRANT EXECUTE ON FUNCTION private.is_lobby_member(uuid, uuid) TO authenticated;
GRANT EXECUTE ON FUNCTION private.are_friends(uuid, uuid) TO authenticated;

-- 2. On repart de zéro : supprime TOUTES les anciennes règles des deux tables
--    (y compris celles ajoutées à la main avec un autre nom).
DO $$
DECLARE pol record;
BEGIN
  FOR pol IN
    SELECT policyname, tablename FROM pg_policies
    WHERE schemaname = 'public' AND tablename IN ('lobbies', 'lobby_members')
  LOOP
    EXECUTE format('DROP POLICY %I ON public.%I', pol.policyname, pol.tablename);
  END LOOP;
END $$;

-- Anciennes copies exposées par l'API (première version de ce script).
DROP FUNCTION IF EXISTS public.is_lobby_host(uuid, uuid);
DROP FUNCTION IF EXISTS public.is_lobby_member(uuid, uuid);
DROP FUNCTION IF EXISTS public.are_friends(uuid, uuid);

ALTER TABLE public.lobbies ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.lobby_members ENABLE ROW LEVEL SECURITY;

-- 3. lobbies
CREATE POLICY "Host and members view the lobby" ON public.lobbies
  FOR SELECT TO authenticated USING (host_id = auth.uid() OR private.is_lobby_member(id, auth.uid()));

CREATE POLICY "Users create their own lobby" ON public.lobbies
  FOR INSERT TO authenticated WITH CHECK (host_id = auth.uid());

CREATE POLICY "Host updates the lobby" ON public.lobbies
  FOR UPDATE TO authenticated USING (host_id = auth.uid()) WITH CHECK (host_id = auth.uid());

CREATE POLICY "Host deletes the lobby" ON public.lobbies
  FOR DELETE TO authenticated USING (host_id = auth.uid());

ALTER TABLE public.lobbies DROP CONSTRAINT IF EXISTS lobbies_netplay_code_length;
ALTER TABLE public.lobbies ADD CONSTRAINT lobbies_netplay_code_length
  CHECK (netplay_code IS NULL OR char_length(netplay_code) <= 2000);

-- 4. lobby_members
CREATE POLICY "Lobby people view its members" ON public.lobby_members
  FOR SELECT TO authenticated USING (
    user_id = auth.uid()
    OR private.is_lobby_member(lobby_id, auth.uid())
    OR private.is_lobby_host(lobby_id, auth.uid())
  );

-- Seul l'hôte ajoute : lui-même, ou un ami accepté (invitation, pas encore prêt).
CREATE POLICY "Host adds itself or invites a friend" ON public.lobby_members
  FOR INSERT TO authenticated WITH CHECK (
    private.is_lobby_host(lobby_id, auth.uid())
    AND (
      user_id = auth.uid()
      OR (private.are_friends(auth.uid(), user_id) AND is_ready = false)
    )
  );

CREATE POLICY "Members update their own row" ON public.lobby_members
  FOR UPDATE TO authenticated USING (user_id = auth.uid()) WITH CHECK (user_id = auth.uid());

CREATE POLICY "Members leave, host removes members" ON public.lobby_members
  FOR DELETE TO authenticated USING (user_id = auth.uid() OR private.is_lobby_host(lobby_id, auth.uid()));

-- Un membre ne peut modifier QUE is_ready (pas déplacer sa ligne vers un
-- autre lobby en changeant lobby_id).
REVOKE UPDATE ON public.lobby_members FROM authenticated, anon;
GRANT UPDATE (is_ready) ON public.lobby_members TO authenticated;

-- 5. Vérification : doit lister exactement 8 règles (4 par table).
SELECT tablename, policyname, cmd FROM pg_policies
WHERE schemaname = 'public' AND tablename IN ('lobbies', 'lobby_members')
ORDER BY tablename, cmd;
