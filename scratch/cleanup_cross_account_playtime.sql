-- ============================================================
-- Nettoyer les duplications de playtime qui ont eu lieu quand
-- un utilisateur s'est connecté sur une machine où un autre
-- compte avait déjà joué. Le fix côté app est en place, mais
-- ces lignes-là existent déjà — à vérifier et supprimer à la main.
--
-- IMPORTANT : lance d'abord les SELECT pour voir ce qui sera supprimé,
-- puis décommente le DELETE correspondant.
-- ============================================================

-- 1) Lister tous les jeux présents chez plusieurs users.
--    Si une même paire (console, name) a exactement les mêmes
--    seconds/launches/last_played sur 2 user_id, c'est quasi
--    sûrement une dup.
select console, name, seconds, launches, last_played, count(*) as users
from public.playtime_games
group by console, name, seconds, launches, last_played
having count(*) > 1;

-- 2) Si tu veux supprimer UNIQUEMENT les rows du compte "ec10665d-..."
--    (le compte où les stats ont atterri à tort), remplace l'UUID par
--    le bon et lance :
-- delete from public.playtime_games
-- where user_id = 'ec10665d-7a4c-42d5-9f29-6c9c0...'
--   and (console, name) in (
--     select console, name
--     from public.playtime_games
--     where user_id = '7ffc9922-7145-44d3-83e1-77e1777...'
--   );

-- Pareil pour les emulators :
-- delete from public.playtime_emulators
-- where user_id = 'ec10665d-7a4c-42d5-9f29-6c9c0...'
--   and emulator_id in (
--     select emulator_id
--     from public.playtime_emulators
--     where user_id = '7ffc9922-7145-44d3-83e1-77e1777...'
--   );

-- 3) Si ce compte ec10665d n'a JAMAIS servi à jouer et que tout
--    son playtime est en réalité dupliqué, supprime tout d'un coup :
-- delete from public.playtime_games      where user_id = 'ec10665d-7a4c-42d5-9f29-6c9c0...';
-- delete from public.playtime_emulators  where user_id = 'ec10665d-7a4c-42d5-9f29-6c9c0...';
