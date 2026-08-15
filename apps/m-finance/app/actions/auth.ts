"use server";

import { headers } from "next/headers";
import { redirect } from "next/navigation";
import { createServerSupabaseClient } from "@/lib/supabase/server";

export async function signInWithGoogle() {
  const supabase = await createServerSupabaseClient();

  if (!supabase) {
    redirect("/login?error=supabase_not_configured");
  }

  const origin = (await headers()).get("origin");
  const redirectTo = `${origin}/api/auth/callback?next=/app/dashboard`;

  const { data, error } = await supabase.auth.signInWithOAuth({
    provider: "google",
    options: {
      redirectTo,
      queryParams: {
        access_type: "offline",
        prompt: "consent",
      },
    },
  });

  if (error || !data.url) {
    redirect("/login?error=oauth");
  }

  redirect(data.url);
}

export async function signOut() {
  const supabase = await createServerSupabaseClient();
  await supabase?.auth.signOut();
  redirect("/login");
}
