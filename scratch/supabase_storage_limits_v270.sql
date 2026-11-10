-- Audit 2.7.0 (2026-11-10): the public buckets accepted any file, of any
-- size. Applied through the Supabase connector with the user's approval.
update storage.buckets
set file_size_limit = 5242880, -- 5 MB
    allowed_mime_types = array['image/png','image/jpeg','image/webp','image/gif']
where id in ('avatars','chat-images');
