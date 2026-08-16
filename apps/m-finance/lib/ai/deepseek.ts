import OpenAI from "openai";
import { env, isDeepSeekConfigured } from "@/lib/env";

let client: OpenAI | null = null;

export function getDeepSeekClient() {
  if (!isDeepSeekConfigured()) {
    return null;
  }

  client ??= new OpenAI({
    apiKey: env.deepseekApiKey,
    baseURL: env.deepseekBaseUrl,
  });

  return client;
}
