ALTER TABLE "bills" ADD COLUMN "series_id" uuid;--> statement-breakpoint
ALTER TABLE "bills" ADD COLUMN "series_number" integer;--> statement-breakpoint
ALTER TABLE "bills" ADD COLUMN "series_total" integer;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD COLUMN "installment_id" uuid;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD COLUMN "installment_number" integer;--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD COLUMN "installment_total" integer;--> statement-breakpoint
ALTER TABLE "bills" ADD CONSTRAINT "bills_user_series_number_unique" UNIQUE("user_id","series_id","series_number");--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD CONSTRAINT "credit_card_expenses_user_installment_number_unique" UNIQUE("user_id","installment_id","installment_number");--> statement-breakpoint
ALTER TABLE "bills" ADD CONSTRAINT "bills_series_valid" CHECK (("bills"."series_id" is null and "bills"."series_number" is null and "bills"."series_total" is null) or ("bills"."series_id" is not null and "bills"."series_number" between 1 and "bills"."series_total" and "bills"."series_total" >= 2));--> statement-breakpoint
ALTER TABLE "credit_card_expenses" ADD CONSTRAINT "credit_card_expenses_installment_valid" CHECK (("credit_card_expenses"."installment_id" is null and "credit_card_expenses"."installment_number" is null and "credit_card_expenses"."installment_total" is null) or ("credit_card_expenses"."installment_id" is not null and "credit_card_expenses"."installment_number" between 1 and "credit_card_expenses"."installment_total" and "credit_card_expenses"."installment_total" >= 2));