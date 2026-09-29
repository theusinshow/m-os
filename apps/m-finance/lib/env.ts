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
  // Secret de LEITURA do M/OS (Intelligence Gateway). Escopo menor que o de
  // acao: le tudo o que o gateway expoe e nao escreve nada (ADR-073).
  mosFinanceReadSecret: process.env.MOS_FINANCE_READ_SECRET ?? "",
  // IA financeira pesada (ADR-073 §10). Qualquer endpoint compativel com a API
  // da OpenAI. Sem chave, `finance.analyze` responde `ai_not_configured` e o
  // Hermes raciocina sobre as ferramentas deterministicas.
  financeAiBaseUrl: process.env.FINANCE_AI_BASE_URL ?? "",
  financeAiApiKey: process.env.FINANCE_AI_API_KEY ?? "",
  financeAiModelStandard: process.env.FINANCE_AI_MODEL_STANDARD ?? "",
  financeAiModelHeavy: process.env.FINANCE_AI_MODEL_HEAVY ?? "",
  financeAiTimeoutMs: Number(process.env.FINANCE_AI_TIMEOUT_MS ?? "45000") || 45000,
  // Narrar insights materiais com a LLM. Desligado, o texto e o do template.
  financeAiNarrateInsights: process.env.FINANCE_AI_NARRATE_INSIGHTS === "true",
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

export function isFinanceAiConfigured() {
  return Boolean(env.financeAiApiKey && env.financeAiBaseUrl && (env.financeAiModelStandard || env.financeAiModelHeavy));
}
