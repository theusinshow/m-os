import { type NextRequest, NextResponse } from "next/server";
import { createServerClient } from "@supabase/ssr";
import { env, isSupabaseConfigured } from "@/lib/env";

export async function proxy(request: NextRequest) {
  const { pathname } = request.nextUrl;

  if (!pathname.startsWith("/app")) {
    return NextResponse.next();
  }

  if (!isSupabaseConfigured()) {
    return NextResponse.redirect(new URL("/login", request.url));
  }

  let response = NextResponse.next({
    request,
  });

  const supabase = createServerClient(env.supabaseUrl, env.supabaseAnonKey, {
    cookies: {
      getAll() {
        return request.cookies.getAll();
      },
      setAll(cookiesToSet) {
        cookiesToSet.forEach(({ name, value }) => request.cookies.set(name, value));
        response = NextResponse.next({ request });
        cookiesToSet.forEach(({ name, value, options }) => {
          response.cookies.set(name, value, options);
        });
      },
    },
  });

  const {
    data: { user },
  } = await supabase.auth.getUser();

  if (!user) {
    return NextResponse.redirect(new URL("/login", request.url));
  }

  if (env.authorizedEmail && user.email !== env.authorizedEmail) {
    return NextResponse.redirect(new URL("/login?error=unauthorized_email", request.url));
  }

  return response;
}

export const config = {
  matcher: ["/app/:path*"],
};
