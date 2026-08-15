import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { env } from "@/lib/env";
import * as schema from "@/db/schema";

const queryClient = env.databaseUrl
  ? postgres(env.databaseUrl, { prepare: false })
  : null;

export const db = queryClient ? drizzle(queryClient, { schema }) : null;
