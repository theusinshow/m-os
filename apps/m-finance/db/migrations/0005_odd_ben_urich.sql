CREATE TYPE "public"."expense_source" AS ENUM('manual', 'openfinance');--> statement-breakpoint
CREATE TYPE "public"."pluggy_item_status" AS ENUM('pending', 'updating', 'ready', 'error');--> statement-breakpoint
CREATE TABLE "pluggy_items" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"card_id" uuid,
	"item_id" text NOT NULL,
	"account_id" text,
	"connector_name" text,
	"status" "pluggy_item_status" DEFAULT 'pending' NOT NULL,
	"last_synced_at" timestamp with time zone,
	"error" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "pluggy_items_user_item_unique" UNIQUE("user_id","item_id")
);
--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD COLUMN "source" "expense_source" DEFAULT 'manual' NOT NULL;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD COLUMN "external_id" text;--> statement-breakpoint
ALTER TABLE "pluggy_items" ADD CONSTRAINT "pluggy_items_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "pluggy_items" ADD CONSTRAINT "pluggy_items_card_id_credit_cards_id_fk" FOREIGN KEY ("card_id") REFERENCES "public"."credit_cards"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD CONSTRAINT "credit_card_expenses_user_external_unique" UNIQUE("user_id","external_id");