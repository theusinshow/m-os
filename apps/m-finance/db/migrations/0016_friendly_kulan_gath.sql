CREATE TYPE "public"."financial_insight_severity" AS ENUM('info', 'warning', 'critical');--> statement-breakpoint
CREATE TYPE "public"."financial_insight_status" AS ENUM('open', 'acknowledged', 'resolved', 'expired');--> statement-breakpoint
CREATE TYPE "public"."financial_insight_type" AS ENUM('bill_due_soon', 'overdue_commitment', 'income_missing', 'card_spending_spike', 'future_month_pressure', 'installment_pressure', 'subscription_load', 'safe_to_spend_drop');--> statement-breakpoint
CREATE TYPE "public"."financial_policy_key" AS ENUM('minimum_month_end_buffer', 'reliable_income_rules', 'max_installment_commitment', 'forecast_horizon_months', 'observer_sensitivity', 'safe_to_spend_policy');--> statement-breakpoint
CREATE TYPE "public"."financial_policy_source" AS ENUM('user', 'hermes', 'whatsapp');--> statement-breakpoint
CREATE TYPE "public"."mos_action_receipt_status" AS ENUM('pending', 'completed');--> statement-breakpoint
CREATE TABLE "financial_insights" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"type" "financial_insight_type" NOT NULL,
	"severity" "financial_insight_severity" NOT NULL,
	"status" "financial_insight_status" DEFAULT 'open' NOT NULL,
	"title" text NOT NULL,
	"summary" text NOT NULL,
	"narrative" text,
	"facts" jsonb NOT NULL,
	"evidence" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"dedupe_key" text NOT NULL,
	"materiality_score" integer NOT NULL,
	"first_seen_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_seen_at" timestamp with time zone DEFAULT now() NOT NULL,
	"acknowledged_at" timestamp with time zone,
	"resolved_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "financial_observer_runs" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"ran_at" timestamp with time zone DEFAULT now() NOT NULL,
	"safe_to_spend_cents" integer NOT NULL,
	"observations" integer NOT NULL,
	"facts" jsonb NOT NULL
);
--> statement-breakpoint
CREATE TABLE "financial_policies" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"key" "financial_policy_key" NOT NULL,
	"value" jsonb NOT NULL,
	"source" "financial_policy_source" DEFAULT 'user' NOT NULL,
	"active" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "financial_policies_user_key_unique" UNIQUE("user_id","key")
);
--> statement-breakpoint
CREATE TABLE "mos_action_receipts" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"idempotency_key" text NOT NULL,
	"action_id" text NOT NULL,
	"status" "mos_action_receipt_status" DEFAULT 'pending' NOT NULL,
	"result" jsonb,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "mos_action_receipts_user_key_unique" UNIQUE("user_id","idempotency_key")
);
--> statement-breakpoint
ALTER TABLE "financial_insights" ADD CONSTRAINT "financial_insights_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "financial_observer_runs" ADD CONSTRAINT "financial_observer_runs_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "financial_policies" ADD CONSTRAINT "financial_policies_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "mos_action_receipts" ADD CONSTRAINT "mos_action_receipts_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "financial_insights_user_dedupe_live_unique" ON "financial_insights" USING btree ("user_id","dedupe_key") WHERE "financial_insights"."status" in ('open', 'acknowledged');--> statement-breakpoint
CREATE INDEX "financial_insights_user_status_idx" ON "financial_insights" USING btree ("user_id","status");--> statement-breakpoint
CREATE INDEX "financial_observer_runs_user_ran_idx" ON "financial_observer_runs" USING btree ("user_id","ran_at");