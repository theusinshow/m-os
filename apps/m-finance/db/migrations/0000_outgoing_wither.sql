CREATE TYPE "public"."alert_entity_type" AS ENUM('bill', 'invoice', 'month');--> statement-breakpoint
CREATE TYPE "public"."alert_severity" AS ENUM('info', 'warning', 'danger');--> statement-breakpoint
CREATE TYPE "public"."alert_type" AS ENUM('due_in_3_days', 'due_today', 'overdue', 'month_review');--> statement-breakpoint
CREATE TYPE "public"."bill_status" AS ENUM('pending', 'paid', 'overdue');--> statement-breakpoint
CREATE TYPE "public"."card_type" AS ENUM('personal', 'business');--> statement-breakpoint
CREATE TYPE "public"."income_type" AS ENUM('main', 'extra', 'freelance');--> statement-breakpoint
CREATE TYPE "public"."invoice_status" AS ENUM('pending', 'paid', 'overdue');--> statement-breakpoint
CREATE TYPE "public"."month_health" AS ENUM('positive', 'fair', 'tight', 'negative');--> statement-breakpoint
CREATE TYPE "public"."month_status" AS ENUM('open', 'closed', 'archived');--> statement-breakpoint
CREATE TABLE "alerts" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"month_id" uuid,
	"alert_type" "alert_type" NOT NULL,
	"entity_type" "alert_entity_type" NOT NULL,
	"entity_id" uuid NOT NULL,
	"title" text NOT NULL,
	"message" text NOT NULL,
	"severity" "alert_severity" NOT NULL,
	"is_read" boolean DEFAULT false NOT NULL,
	"trigger_date" date NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "bill_categories" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"slug" text NOT NULL,
	"color" text,
	"icon" text,
	"is_default" boolean DEFAULT false NOT NULL,
	"is_archived" boolean DEFAULT false NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "bill_categories_user_slug_unique" UNIQUE("user_id","slug")
);
--> statement-breakpoint
CREATE TABLE "bills" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"month_id" uuid NOT NULL,
	"category_id" uuid,
	"recurrence_rule_id" uuid,
	"name" text NOT NULL,
	"amount_cents" integer NOT NULL,
	"due_date" date NOT NULL,
	"is_recurring" boolean DEFAULT false NOT NULL,
	"status" "bill_status" DEFAULT 'pending' NOT NULL,
	"notes" text,
	"paid_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "bills_amount_positive" CHECK ("bills"."amount_cents" > 0)
);
--> statement-breakpoint
CREATE TABLE "credit_card_invoices" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"card_id" uuid NOT NULL,
	"month_id" uuid NOT NULL,
	"amount_cents" integer NOT NULL,
	"due_date" date NOT NULL,
	"status" "invoice_status" DEFAULT 'pending' NOT NULL,
	"paid_at" timestamp with time zone,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "credit_card_invoices_card_month_unique" UNIQUE("card_id","month_id"),
	CONSTRAINT "credit_card_invoices_amount_positive" CHECK ("credit_card_invoices"."amount_cents" > 0)
);
--> statement-breakpoint
CREATE TABLE "credit_cards" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"card_type" "card_type" DEFAULT 'personal' NOT NULL,
	"due_day" integer NOT NULL,
	"is_active" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "credit_cards_due_day_range" CHECK ("credit_cards"."due_day" between 1 and 31)
);
--> statement-breakpoint
CREATE TABLE "incomes" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"month_id" uuid NOT NULL,
	"name" text NOT NULL,
	"amount_cents" integer NOT NULL,
	"income_type" "income_type" NOT NULL,
	"expected_date" date,
	"received" boolean DEFAULT false NOT NULL,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "incomes_amount_positive" CHECK ("incomes"."amount_cents" > 0)
);
--> statement-breakpoint
CREATE TABLE "monthly_snapshots" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"month_id" uuid NOT NULL,
	"total_income_cents" integer NOT NULL,
	"total_bills_cents" integer NOT NULL,
	"total_invoices_cents" integer NOT NULL,
	"total_paid_cents" integer NOT NULL,
	"total_pending_cents" integer NOT NULL,
	"total_overdue_cents" integer NOT NULL,
	"estimated_remaining_cents" integer NOT NULL,
	"month_health" "month_health" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "months" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"month" integer NOT NULL,
	"year" integer NOT NULL,
	"status" "month_status" DEFAULT 'open' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "months_user_month_year_unique" UNIQUE("user_id","month","year"),
	CONSTRAINT "months_month_range" CHECK ("months"."month" between 1 and 12),
	CONSTRAINT "months_year_min" CHECK ("months"."year" >= 2020)
);
--> statement-breakpoint
CREATE TABLE "recurrence_rules" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"category_id" uuid,
	"default_amount_cents" integer,
	"due_day" integer NOT NULL,
	"is_variable_amount" boolean DEFAULT false NOT NULL,
	"is_active" boolean DEFAULT true NOT NULL,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "recurrence_rules_due_day_range" CHECK ("recurrence_rules"."due_day" between 1 and 31),
	CONSTRAINT "recurrence_rules_default_amount_positive" CHECK ("recurrence_rules"."default_amount_cents" is null or "recurrence_rules"."default_amount_cents" > 0)
);
--> statement-breakpoint
CREATE TABLE "settings" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"alert_days_before" integer DEFAULT 3 NOT NULL,
	"theme" text DEFAULT 'dark' NOT NULL,
	"currency" text DEFAULT 'BRL' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "settings_user_id_unique" UNIQUE("user_id")
);
--> statement-breakpoint
CREATE TABLE "users" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"supabase_user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"email" text NOT NULL,
	"avatar_url" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "users_supabase_user_id_unique" UNIQUE("supabase_user_id"),
	CONSTRAINT "users_email_unique" UNIQUE("email")
);
--> statement-breakpoint
ALTER TABLE "alerts" ADD CONSTRAINT "alerts_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "alerts" ADD CONSTRAINT "alerts_month_id_months_id_fk" FOREIGN KEY ("month_id") REFERENCES "public"."months"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "bill_categories" ADD CONSTRAINT "bill_categories_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "bills" ADD CONSTRAINT "bills_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "bills" ADD CONSTRAINT "bills_month_id_months_id_fk" FOREIGN KEY ("month_id") REFERENCES "public"."months"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "bills" ADD CONSTRAINT "bills_category_id_bill_categories_id_fk" FOREIGN KEY ("category_id") REFERENCES "public"."bill_categories"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "bills" ADD CONSTRAINT "bills_recurrence_rule_id_recurrence_rules_id_fk" FOREIGN KEY ("recurrence_rule_id") REFERENCES "public"."recurrence_rules"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "credit_card_invoices" ADD CONSTRAINT "credit_card_invoices_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "credit_card_invoices" ADD CONSTRAINT "credit_card_invoices_card_id_credit_cards_id_fk" FOREIGN KEY ("card_id") REFERENCES "public"."credit_cards"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "credit_card_invoices" ADD CONSTRAINT "credit_card_invoices_month_id_months_id_fk" FOREIGN KEY ("month_id") REFERENCES "public"."months"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "credit_cards" ADD CONSTRAINT "credit_cards_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "incomes" ADD CONSTRAINT "incomes_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "incomes" ADD CONSTRAINT "incomes_month_id_months_id_fk" FOREIGN KEY ("month_id") REFERENCES "public"."months"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "monthly_snapshots" ADD CONSTRAINT "monthly_snapshots_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "monthly_snapshots" ADD CONSTRAINT "monthly_snapshots_month_id_months_id_fk" FOREIGN KEY ("month_id") REFERENCES "public"."months"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "months" ADD CONSTRAINT "months_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "recurrence_rules" ADD CONSTRAINT "recurrence_rules_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "recurrence_rules" ADD CONSTRAINT "recurrence_rules_category_id_bill_categories_id_fk" FOREIGN KEY ("category_id") REFERENCES "public"."bill_categories"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "settings" ADD CONSTRAINT "settings_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "users" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "months" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "incomes" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "bill_categories" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "recurrence_rules" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "bills" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "credit_cards" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "credit_card_invoices" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "alerts" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "monthly_snapshots" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
ALTER TABLE "settings" ENABLE ROW LEVEL SECURITY;--> statement-breakpoint
CREATE POLICY "users_select_own" ON "users" FOR SELECT USING ("supabase_user_id" = auth.uid());--> statement-breakpoint
CREATE POLICY "users_insert_own" ON "users" FOR INSERT WITH CHECK ("supabase_user_id" = auth.uid());--> statement-breakpoint
CREATE POLICY "users_update_own" ON "users" FOR UPDATE USING ("supabase_user_id" = auth.uid()) WITH CHECK ("supabase_user_id" = auth.uid());--> statement-breakpoint
CREATE POLICY "months_owner_all" ON "months" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "months"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "months"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "incomes_owner_all" ON "incomes" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "incomes"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "incomes"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "bill_categories_owner_all" ON "bill_categories" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "bill_categories"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "bill_categories"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "recurrence_rules_owner_all" ON "recurrence_rules" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "recurrence_rules"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "recurrence_rules"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "bills_owner_all" ON "bills" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "bills"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "bills"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "credit_cards_owner_all" ON "credit_cards" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "credit_cards"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "credit_cards"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "credit_card_invoices_owner_all" ON "credit_card_invoices" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "credit_card_invoices"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "credit_card_invoices"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "alerts_owner_all" ON "alerts" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "alerts"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "alerts"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "monthly_snapshots_owner_all" ON "monthly_snapshots" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "monthly_snapshots"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "monthly_snapshots"."user_id" AND "users"."supabase_user_id" = auth.uid()));--> statement-breakpoint
CREATE POLICY "settings_owner_all" ON "settings" FOR ALL USING (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "settings"."user_id" AND "users"."supabase_user_id" = auth.uid())) WITH CHECK (EXISTS (SELECT 1 FROM "users" WHERE "users"."id" = "settings"."user_id" AND "users"."supabase_user_id" = auth.uid()));
