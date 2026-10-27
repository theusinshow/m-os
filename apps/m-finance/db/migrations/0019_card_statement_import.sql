ALTER TYPE "public"."expense_source" ADD VALUE 'statement';--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD COLUMN "category_id" uuid;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD COLUMN "subscription_id" uuid;--> statement-breakpoint
ALTER TABLE "credit_card_invoices" ADD COLUMN "credits_cents" integer DEFAULT 0 NOT NULL;--> statement-breakpoint
ALTER TABLE "subscriptions" ADD COLUMN "card_id" uuid;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD CONSTRAINT "credit_card_expenses_category_id_bill_categories_id_fk" FOREIGN KEY ("category_id") REFERENCES "public"."bill_categories"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD CONSTRAINT "credit_card_expenses_subscription_id_subscriptions_id_fk" FOREIGN KEY ("subscription_id") REFERENCES "public"."subscriptions"("id") ON DELETE set null ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "subscriptions" ADD CONSTRAINT "subscriptions_card_id_credit_cards_id_fk" FOREIGN KEY ("card_id") REFERENCES "public"."credit_cards"("id") ON DELETE set null ON UPDATE no action;