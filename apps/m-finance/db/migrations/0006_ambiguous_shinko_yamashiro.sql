CREATE TYPE "public"."subscription_cycle" AS ENUM('once', 'monthly', 'yearly');--> statement-breakpoint
CREATE TYPE "public"."subscription_status" AS ENUM('trial', 'active', 'canceled');--> statement-breakpoint
CREATE TABLE "push_subscriptions" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"endpoint" text NOT NULL,
	"p256dh" text NOT NULL,
	"auth" text NOT NULL,
	"user_agent" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "push_subscriptions_endpoint_unique" UNIQUE("endpoint")
);
--> statement-breakpoint
CREATE TABLE "subscriptions" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"amount_cents" integer NOT NULL,
	"next_charge_date" date NOT NULL,
	"cycle" "subscription_cycle" DEFAULT 'monthly' NOT NULL,
	"status" "subscription_status" DEFAULT 'trial' NOT NULL,
	"reminder_days_before" integer DEFAULT 1 NOT NULL,
	"last_notified_for" date,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "subscriptions_amount_positive" CHECK ("subscriptions"."amount_cents" > 0),
	CONSTRAINT "subscriptions_reminder_days_range" CHECK ("subscriptions"."reminder_days_before" between 0 and 30)
);
--> statement-breakpoint
ALTER TABLE "push_subscriptions" ADD CONSTRAINT "push_subscriptions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "subscriptions" ADD CONSTRAINT "subscriptions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;