import { NextResponse, type NextRequest } from "next/server";
import { createServerSupabaseClient } from "@/lib/supabase/server";
import { env } from "@/lib/env";

export async function GET(request: NextRequest) {
  const requestUrl = new URL(request.url);
  const code = requestUrl.searchParams.get("code");
  const next = requestUrl.searchParams.get("next") ?? "/app/dashboard";

  if (!code) {
    return NextResponse.redirect(new URL("/login?error=missing_code", request.url));
  }

  const supabase = await createServerSupabaseClient();

  if (!supabase) {
    return NextResponse.redirect(new URL("/login?error=supabase_not_configured", request.url));
  }

  const { error } = await supabase.auth.exchangeCodeForSession(code);

  if (error) {
    return NextResponse.redirect(new URL("/login?error=callback", request.url));
  }

  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (env.authorizedEmail && user?.email !== env.authorizedEmail) {
    await supabase.auth.signOut();
    return NextResponse.redirect(new URL("/login?error=unauthorized_email", request.url));
  }

  return NextResponse.redirect(new URL(next, request.url));
}
