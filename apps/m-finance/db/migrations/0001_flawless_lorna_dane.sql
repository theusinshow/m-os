CREATE TYPE "public"."payment_type" AS ENUM('cash', 'installment');--> statement-breakpoint
CREATE TYPE "public"."risk_level" AS ENUM('safe', 'controlled', 'tight', 'critical');--> statement-breakpoint
CREATE TABLE "purchase_simulations" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"total_amount_cents" integer NOT NULL,
	"payment_type" "payment_type" NOT NULL,
	"installments" integer,
	"start_month" integer NOT NULL,
	"start_year" integer NOT NULL,
	"monthly_impact_cents" integer NOT NULL,
	"risk_level" "risk_level" NOT NULL,
	"recommendation" text NOT NULL,
	"result_payload" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "purchase_simulations_total_positive" CHECK ("purchase_simulations"."total_amount_cents" > 0),
	CONSTRAINT "purchase_simulations_start_month_range" CHECK ("purchase_simulations"."start_month" between 1 and 12),
	CONSTRAINT "purchase_simulations_installments_valid" CHECK ("purchase_simulations"."installments" is null or "purchase_simulations"."installments" >= 1)
);
--> statement-breakpoint
ALTER TABLE "purchase_simulations" ADD CONSTRAINT "purchase_simulations_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;