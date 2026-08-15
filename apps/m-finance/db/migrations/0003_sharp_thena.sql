CREATE TABLE "credit_card_expenses" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"card_id" uuid NOT NULL,
	"month_id" uuid NOT NULL,
	"description" text NOT NULL,
	"amount_cents" integer NOT NULL,
	"purchase_date" date,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "credit_card_expenses_amount_positive" CHECK ("credit_card_expenses"."amount_cents" > 0)
);
--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD CONSTRAINT "credit_card_expenses_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD CONSTRAINT "credit_card_expenses_card_id_credit_cards_id_fk" FOREIGN KEY ("card_id") REFERENCES "public"."credit_cards"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD CONSTRAINT "credit_card_expenses_month_id_months_id_fk" FOREIGN KEY ("month_id") REFERENCES "public"."months"("id") ON DELETE cascade ON UPDATE no action;