import twilio from "twilio";

export function createWhatsappXmlResponse(message: string) {
  const response = new twilio.twiml.MessagingResponse();
  response.message(message);

  return new Response(response.toString(), {
    status: 200,
    headers: {
      "Content-Type": "text/xml; charset=utf-8",
    },
  });
}

export function createEmptyWhatsappResponse(status = 200) {
  return new Response("", { status });
}

export const WHATSAPP_HELP_MESSAGE = [
  "M Finance no WhatsApp",
  "",
  "Consultas:",
  "• ajuda",
  "• resumo / saldo / gastos",
  "• vencimentos",
  "• vencidas",
  "• comparacao (vs mês anterior)",
  "• quanto gastei no nubank / fatura do itaú",
  "• fatura do nubank de agosto (mês específico)",
  "",
  "Lançamentos (com confirmação):",
  "• gastei 32 no almoço no nubank pessoal",
  "• comprei 600 na amazon em 6x no itaú",
  "• paguei 120 de conta de luz",
  "• todo mês tenho 49,90 de spotify dia 10",
  "• gastei 32 ontem no almoço",
  "",
  "Ações:",
  "• paguei a conta de luz (marca como paga)",
  "• paguei a fatura do itaú (marca fatura como paga)",
  "• edita a última compra pra 50 (edita valor)",
  "• cancela a última compra (desfaz)",
  "",
  "Responda sim ou não para confirmar/cancelar.",
].join("\n");
