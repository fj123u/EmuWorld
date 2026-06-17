// Edge Function: supabase/functions/delete-account/index.ts
// Deploy with: supabase functions deploy delete-account
// Set SUPABASE_SERVICE_ROLE_KEY as a secret in Supabase dashboard

import { serve } from "https://deno.land/std@0.168.0/http/server.ts";
import { createClient } from "https://esm.sh/@supabase/supabase-js@2";

serve(async (req) => {
  const authHeader = req.headers.get("Authorization");
  if (!authHeader) {
    return new Response(JSON.stringify({ error: "Missing auth" }), { status: 401 });
  }

  const supabaseUrl = Deno.env.get("SUPABASE_URL")!;
  const serviceRoleKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;

  // Verify the user's JWT
  const userClient = createClient(supabaseUrl, Deno.env.get("SUPABASE_ANON_KEY")!, {
    global: { headers: { Authorization: authHeader } },
  });
  const { data: { user }, error: userError } = await userClient.auth.getUser();
  if (userError || !user) {
    return new Response(JSON.stringify({ error: "Invalid token" }), { status: 401 });
  }

  // Use service role to delete all user data then the auth user
  const adminClient = createClient(supabaseUrl, serviceRoleKey);

  // Delete user data from all tables
  const tables = [
    "playtime_games",
    "user_achievements",
    "activity_feed",
    "messages",
    "friends",
    "lobby_members",
    "lobbies",
    "versus_challenges",
    "reviews",
    "guides",
    "themes",
    "collections",
  ];

  for (const table of tables) {
    await adminClient.from(table).delete().eq("user_id", user.id);
  }
  // Messages where user is sender or receiver
  await adminClient.from("messages").delete().eq("sender_id", user.id);
  await adminClient.from("messages").delete().eq("receiver_id", user.id);
  // Friends where user is either side
  await adminClient.from("friends").delete().eq("requester_id", user.id);
  await adminClient.from("friends").delete().eq("addressee_id", user.id);

  // Delete profile
  await adminClient.from("profiles").delete().eq("id", user.id);

  // Delete the auth user
  const { error: deleteError } = await adminClient.auth.admin.deleteUser(user.id);
  if (deleteError) {
    return new Response(JSON.stringify({ error: deleteError.message }), { status: 500 });
  }

  return new Response(JSON.stringify({ success: true }), { status: 200 });
});
