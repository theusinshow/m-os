import { and, desc, eq, sql } from "drizzle-orm";
import twilio from "twilio";
import { db } from "@/db/client";
import { whatsappMessages } from "@/db/schema";
import { env, isTwilioConfigured } from "@/lib/env";
import { logWhatsappMessage } from "@/lib/whatsapp/audit";

let client: twilio.Twilio | null = null;

function getTwilioClient() {
  if (!isTwilioConfigured()) return null;
  client ??= twilio(env.twilioAccountSid, env.twilioAuthToken);
  return client;
}

const TWENTY_FOUR_HOURS_MS = 24 * 60 * 60 * 1000;

/**
 * WhatsApp Business só permite mensagens livres (não-template) dentro de 24h
 * após a última mensagem inbound do usuário. No Sandbox isso é aplicado à risca.
 * Consultamos o último inbound registrado para decidir se podemos enviar.
 */
export async function isWithinWhatsappWindow(phone: string): Promise<boolean> {
  if (!db) return false;
  const [lastInbound] = await db
    .select({ createdAt: whatsappMessages.createdAt })
    .from(whatsappMessages)
    .where(and(eq(whatsappMessages.from, phone), eq(whatsappMessages.direction, "inbound")))
    .orderBy(desc(whatsappMessages.createdAt))
    .limit(1);

  if (!lastInbound) return false;
  return Date.now() - lastInbound.createdAt.getTime() <= TWENTY_FOUR_HOURS_MS;
}

/**
 * Guarda de idempotência: evita reenviar a mesma notificação se o cron rodar de
 * novo no mesmo dia. A chave é guardada no metadata jsonb da mensagem outbound.
 */
export async function wasWhatsappNotificationSent(notificationKey: string): Promise<boolean> {
  if (!db) return false;
  const [existing] = await db
    .select({ id: whatsappMessages.id })
    .from(whatsappMessages)
    .where(sql`${whatsappMessages.metadata}->>'notificationKey' = ${notificationKey}`)
    .limit(1);
  return Boolean(existing);
}

type SendResult = { ok: true; sid?: string } | { ok: false; error: string };

/**
 * Envia uma mensagem WhatsApp via Twilio REST API (saída proativa, não TwiML).
 * Retorna o sucesso/falha para o chamador decidir o log.
 */
export async function sendWhatsappMessage(to: string, body: string): Promise<SendResult> {
  const twilioClient = getTwilioClient();
  if (!twilioClient) return { ok: false, error: "twilio_not_configured" };

  try {
    const message = await twilioClient.messages.create({
      from: env.twilioWhatsappFrom,
      to,
      body,
    });
    return { ok: true, sid: message.sid };
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : "unknown_error" };
  }
}

/**
 * Envio de notificação com log de auditoria embutido. Centraliza o padrão de
 * registrar a mensagem outbound e a chave de idempotência.
 */
export async function sendWhatsappNotification(input: {
  userId: string;
  to: string;
  body: string;
  notificationKey: string;
}): Promise<SendResult> {
  const result = await sendWhatsappMessage(input.to, input.body);

  await logWhatsappMessage({
    userId: input.userId,
    direction: "outbound",
    status: result.ok ? "sent" : "error",
    from: env.twilioWhatsappFrom,
    to: input.to,
    body: input.body,
    error: result.ok ? null : result.error,
    metadata: { notificationKey: input.notificationKey },
  });

  return result;
}

type ButtonSendResult =
  | { ok: true; sentVia: "buttons" | "text"; sid?: string }
  | { ok: false; sentVia: "text"; error: string; fallbackText: string };

/**
 * Tenta enviar uma confirmação com botões "Sim" e "Não" usando um template
 * aprovado da Meta (Twilio Content API). Se o template não estiver configurado
 * ou o envio falhar (Sandbox/sender sem aprovação), devolve o texto de fallback
 * para o chamador responder via TwiML — garantindo que o usuário sempre tenha
 * como confirmar.
 *
 * O `body` já contém o resumo + "Responda sim ou não", então serve como fallback
 * textual direto.
 */
export async function sendConfirmationButtons(input: {
  to: string;
  body: string;
}): Promise<ButtonSendResult> {
  const twilioClient = getTwilioClient();
  const templateSid = env.whatsappConfirmTemplateSid;

  if (!twilioClient || !templateSid) {
    return { ok: false, sentVia: "text", error: "not_configured", fallbackText: input.body };
  }

  try {
    const message = await twilioClient.messages.create({
      from: env.twilioWhatsappFrom,
      to: input.to,
      contentSid: templateSid,
      contentVariables: JSON.stringify({ 1: input.body }),
    });
    return { ok: true, sentVia: "buttons", sid: message.sid };
  } catch (error) {
    return {
      ok: false,
      sentVia: "text",
      error: error instanceof Error ? error.message : "unknown_error",
      fallbackText: input.body,
    };
  }
}
