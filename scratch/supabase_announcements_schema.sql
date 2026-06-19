-- ============================================
-- EmuWorld — Announcements system
-- Allows pushing messages to all users from Supabase dashboard
-- Run in Supabase SQL Editor
-- ============================================

CREATE TABLE IF NOT EXISTS announcements (
  id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
  title text NOT NULL,
  message text NOT NULL,
  type text NOT NULL DEFAULT 'info' CHECK (type IN ('info', 'warning', 'critical')),
  link_url text,
  link_label text,
  min_version text,
  max_version text,
  active boolean DEFAULT true,
  created_at timestamptz DEFAULT now() NOT NULL,
  expires_at timestamptz
);

-- RLS
ALTER TABLE announcements ENABLE ROW LEVEL SECURITY;

-- Anyone authenticated can read active announcements
CREATE POLICY "Authenticated read announcements" ON announcements
  FOR SELECT USING (auth.uid() IS NOT NULL AND active = true);

-- No INSERT/UPDATE/DELETE from client — manage via Supabase dashboard only
