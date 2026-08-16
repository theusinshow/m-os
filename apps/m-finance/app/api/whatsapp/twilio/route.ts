import { db } from "@/db/client";
import { env } from "@/lib/env";
import {
  getWhatsappOwnerUser,
  isAllowedWhatsappSender,
  isAuthorizedWhatsappWebhook,
} from "@/lib/whatsapp/auth";
import { logWhatsappMessage } from "@/lib/whatsapp/audit";
import { handleWhatsappCommand } from "@/lib/whatsapp/commands";
import {
  createEmptyWhatsappResponse,
  createWhatsappXmlResponse,
} from "@/lib/whatsapp/responses";
import { sendConfirmationButtons } from "@/lib/whatsapp/twilio-outbound";

export const runtime = "nodejs";

type TwilioWhatsappPayload = {
  From?: string;
  To?: string;
  Body?: string;
  MessageSid?: string;
};

function parseTwilioPayload(formData: FormData): TwilioWhatsappPayload {
  return {
    From: String(formData.get("From") ?? ""),
    To: String(formData.get("To") ?? ""),
    Body: String(formData.get("Body") ?? ""),
    MessageSid: String(formData.get("MessageSid") ?? ""),
  };
}

/**
 * Twilio WhatsApp inbound webhook.
 *
 * Configure the Twilio sandbox/number webhook as:
 * https://<host>/api/whatsapp/twilio?secret=<WHATSAPP_WEBHOOK_SECRET>
 */
export async function POST(request: Request) {
  if (!isAuthorizedWhatsappWebhook(request)) {
    return new Response("Unauthorized", { status: 401 });
  }

  if (!db) {
    return createWhatsappXmlResponse("Banco de dados indisponível no momento.");
  }

  const payload = parseTwilioPayload(await request.formData());

  if (!isAllowedWhatsappSender(payload.From)) {
    await logWhatsappMessage({
      direction: "inbound",
      status: "ignored",
      from: payload.From,
      to: payload.To,
      body: payload.Body,
      twilioMessageSid: payload.MessageSid,
      error: "sender_not_allowed",
    });

    return createEmptyWhatsappResponse();
  }

  const user = await getWhatsappOwnerUser();

  if (!user) {
    const response = "Usuário autorizado não encontrado. Faça login no app antes de usar o WhatsApp.";

    await logWhatsappMessage({
      direction: "inbound",
      status: "error",
      from: payload.From,
      to: payload.To,
      body: payload.Body,
      twilioMessageSid: payload.MessageSid,
      error: "authorized_user_not_found",
    });

    await logWhatsappMessage({
      direction: "outbound",
      status: "sent",
      from: payload.To,
      to: payload.From,
      body: response,
      error: "authorized_user_not_found",
    });

    return createWhatsappXmlResponse(response);
  }

  await logWhatsappMessage({
    userId: user.id,
    direction: "inbound",
    status: "received",
    from: payload.From,
    to: payload.To,
    body: payload.Body,
    twilioMessageSid: payload.MessageSid,
  });

  let response: string;

  try {
    response = await handleWhatsappCommand({
      message: payload.Body ?? "",
      phone: payload.From ?? "",
      userId: user.id,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : "unknown_error";

    await logWhatsappMessage({
      userId: user.id,
      direction: "outbound",
      status: "error",
      from: payload.To,
      to: payload.From,
      body: "Não consegui processar sua mensagem agora.",
      error: message,
    });

    return createWhatsappXmlResponse("Não consegui processar sua mensagem agora.");
  }

  await logWhatsappMessage({
    userId: user.id,
    direction: "outbound",
    status: "sent",
    from: payload.To,
    to: payload.From,
    body: response,
  });

  const isConfirmation = response.includes("Responda sim ou nao") || response.includes("Responda sim ou não");

  if (isConfirmation && env.whatsappConfirmTemplateSid) {
    const buttonResult = await sendConfirmationButtons({
      to: payload.From ?? "",
      body: response,
    });

    if (buttonResult.ok) {
      await logWhatsappMessage({
        userId: user.id,
        direction: "outbound",
        status: "sent",
        from: payload.To,
        to: payload.From,
        body: response,
        twilioMessageSid: buttonResult.sid ?? null,
        metadata: { sentVia: buttonResult.sentVia },
      });
      return createEmptyWhatsappResponse();
    }
  }

  return createWhatsappXmlResponse(response);
}
