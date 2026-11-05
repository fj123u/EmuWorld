-- ============================================
-- EmuWorld — Notes de jeu privées (06/11/2026)
-- Appliqué via le connecteur Supabase ; rejouable sans risque.
-- ============================================
-- playtime_games est lisible par tous pour un profil public (stats,
-- classement, web panel), et la colonne notes (notes perso : codes, où on en
-- est…) partait avec. Les notes vivent maintenant dans playtime_notes,
-- lisible uniquement par son propriétaire. La colonne playtime_games.notes
-- reste (toujours vide) pour ne pas casser les versions déjà installées :
-- un déclencheur range dans la table privée ce qu'elles y écrivent encore.

CREATE TABLE IF NOT EXISTS public.playtime_notes (
  user_id uuid NOT NULL DEFAULT auth.uid() REFERENCES auth.users(id) ON DELETE CASCADE,
  console text NOT NULL,
  name text NOT NULL,
  notes text NOT NULL CHECK (char_length(notes) <= 5000),
  updated_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (user_id, console, name)
);

ALTER TABLE public.playtime_notes ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "Owner manages own notes" ON public.playtime_notes;
CREATE POLICY "Owner manages own notes" ON public.playtime_notes
  FOR ALL TO authenticated
  USING (user_id = auth.uid())
  WITH CHECK (user_id = auth.uid());
REVOKE ALL ON public.playtime_notes FROM anon;

-- Anciennes versions de l'app : la note envoyée dans playtime_games part dans
-- la table privée et n'est jamais stockée (ni lisible) côté public.
CREATE OR REPLACE FUNCTION public.playtime_games_keep_notes_private()
RETURNS trigger LANGUAGE plpgsql SET search_path = public AS $$
BEGIN
  IF NEW.notes IS NOT NULL AND btrim(NEW.notes) <> '' THEN
    INSERT INTO public.playtime_notes (user_id, console, name, notes, updated_at)
    VALUES (NEW.user_id, NEW.console, NEW.name, left(NEW.notes, 5000), now())
    ON CONFLICT (user_id, console, name) DO UPDATE SET notes = EXCLUDED.notes, updated_at = now();
  END IF;
  NEW.notes := NULL;
  RETURN NEW;
END $$;

DROP TRIGGER IF EXISTS playtime_games_keep_notes_private ON public.playtime_games;
CREATE TRIGGER playtime_games_keep_notes_private
  BEFORE INSERT OR UPDATE ON public.playtime_games
  FOR EACH ROW EXECUTE FUNCTION public.playtime_games_keep_notes_private();

-- Notes existantes : copiées dans la table privée puis effacées du public.
INSERT INTO public.playtime_notes (user_id, console, name, notes)
SELECT user_id, console, name, left(notes, 5000) FROM public.playtime_games
WHERE notes IS NOT NULL AND btrim(notes) <> ''
ON CONFLICT (user_id, console, name) DO UPDATE SET notes = EXCLUDED.notes, updated_at = now();

UPDATE public.playtime_games SET notes = NULL WHERE notes IS NOT NULL;

NOTIFY pgrst, 'reload schema';
