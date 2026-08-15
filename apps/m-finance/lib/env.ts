export const env = {
  supabaseUrl: process.env.NEXT_PUBLIC_SUPABASE_URL ?? "",
  supabaseAnonKey: process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY ?? "",
  authorizedEmail: process.env.AUTHORIZED_EMAIL ?? "",
  databaseUrl: process.env.DATABASE_URL ?? "",
  // Open Finance via Pluggy (official SDK handles auth + base URL).
  pluggyClientId: process.env.PLUGGY_CLIENT_ID ?? "",
  pluggyClientSecret: process.env.PLUGGY_CLIENT_SECRET ?? "",
  // Shared secret required on the webhook URL so only Pluggy can trigger syncs.
  pluggyWebhookSecret: process.env.PLUGGY_WEBHOOK_SECRET ?? "",
  // Web Push (VAPID). Private key stays server-side; public is also exposed via
  // NEXT_PUBLIC_VAPID_PUBLIC_KEY for the browser subscription call.
  vapidPublicKey: process.env.VAPID_PUBLIC_KEY ?? "",
  vapidPrivateKey: process.env.VAPID_PRIVATE_KEY ?? "",
  vapidSubject: process.env.VAPID_SUBJECT ?? "mailto:admin@example.com",
  // Secret Vercel Cron sends so only it can trigger the daily reminder run.
  cronSecret: process.env.CRON_SECRET ?? "",
};

export function isSupabaseConfigured() {
  if (!env.supabaseUrl || !env.supabaseAnonKey) {
    return false;
  }

  try {
    const url = new URL(env.supabaseUrl);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export function isPluggyConfigured() {
  return Boolean(env.pluggyClientId && env.pluggyClientSecret);
}

export function isPushConfigured() {
  return Boolean(env.vapidPublicKey && env.vapidPrivateKey);
}
