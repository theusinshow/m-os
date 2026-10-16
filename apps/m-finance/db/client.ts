import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { env } from "@/lib/env";
import * as schema from "@/db/schema";

// Dados passam por esta conexão direta (DATABASE_URL). No Supabase ela usa
// o role postgres, que ignora RLS. As policies de
// db/migrations/0018_enable_rls.sql valem para o Data API
// (anon/authenticated). Mantenha este cliente num role com BYPASSRLS:
// a sessão não carrega auth.uid(), então um role sujeito a RLS veria zero linhas.
const queryClient = env.databaseUrl
  ? postgres(env.databaseUrl, { prepare: false })
  : null;

export const db = queryClient ? drizzle(queryClient, { schema }) : null;
