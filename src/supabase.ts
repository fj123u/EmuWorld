import { createClient } from '@supabase/supabase-js';

const SUPABASE_URL = 'https://yizxrntlerzfniqkdvfg.supabase.co';
const SUPABASE_ANON_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InlpenhybnRsZXJ6Zm5pcWtkdmZnIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzQ4OTU1NjcsImV4cCI6MjA5MDQ3MTU2N30.cfZhqeB8IntRnefIS3DirXzwIJ3-u5XZPPXr7g1rO-w';

export const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY, {
  auth: {
    flowType: 'pkce',
    autoRefreshToken: true,
    persistSession: true,
    detectSessionInUrl: false, // We handle deep links manually
  },
});

export type Profile = {
  id: string;
  username: string | null;
  avatar_url: string | null;
  full_name: string | null;
  updated_at: string | null;
};
