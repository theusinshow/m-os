export const env = {
  supabaseUrl: process.env.NEXT_PUBLIC_SUPABASE_URL ?? "",
  supabaseAnonKey: process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY ?? "",
  authorizedEmail: process.env.AUTHORIZED_EMAIL ?? "",
  databaseUrl: process.env.DATABASE_URL ?? "",
  // Web Push (VAPID). Private key stays server-side; public is also exposed via
  // NEXT_PUBLIC_VAPID_PUBLIC_KEY for the browser subscription call.
  vapidPublicKey: process.env.VAPID_PUBLIC_KEY ?? "",
  vapidPrivateKey: process.env.VAPID_PRIVATE_KEY ?? "",
  vapidSubject: process.env.VAPID_SUBJECT ?? "mailto:admin@example.com",
  // Secret Vercel Cron sends so only it can trigger the daily reminder run.
  cronSecret: process.env.CRON_SECRET ?? "",
  // Secret que autoriza o M/OS a chamar a Action API (Hermes propondo acoes).
  mosActionSecret: process.env.MOS_ACTION_SECRET ?? "",
  // WhatsApp via Twilio. The webhook is intentionally private to one phone.
  twilioAccountSid: process.env.TWILIO_ACCOUNT_SID ?? "",
  twilioAuthToken: process.env.TWILIO_AUTH_TOKEN ?? "",
  twilioWhatsappFrom: process.env.TWILIO_WHATSAPP_FROM ?? "",
  whatsappAllowedPhone: process.env.WHATSAPP_ALLOWED_PHONE ?? "",
  whatsappWebhookSecret: process.env.WHATSAPP_WEBHOOK_SECRET ?? "",
  // Template SID (Twilio Content API) aprovado pela Meta com botões "Sim" e
  // "Não". Opcional: sem ele, as confirmações seguem como texto via TwiML.
  whatsappConfirmTemplateSid: process.env.WHATSAPP_CONFIRM_TEMPLATE_SID ?? "",
  // DeepSeek uses an OpenAI-compatible API.
  deepseekApiKey: process.env.DEEPSEEK_API_KEY ?? "",
  deepseekBaseUrl: process.env.DEEPSEEK_BASE_URL ?? "https://api.deepseek.com",
  deepseekModel: process.env.DEEPSEEK_MODEL ?? "deepseek-v4-flash",
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

export function isPushConfigured() {
  return Boolean(env.vapidPublicKey && env.vapidPrivateKey);
}

export function isTwilioConfigured() {
  return Boolean(env.twilioAccountSid && env.twilioAuthToken && env.twilioWhatsappFrom);
}

export function isDeepSeekConfigured() {
  return Boolean(env.deepseekApiKey && env.deepseekBaseUrl && env.deepseekModel);
}
