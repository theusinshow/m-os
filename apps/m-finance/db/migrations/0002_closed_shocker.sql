CREATE TYPE "public"."goal_priority" AS ENUM('low', 'medium', 'high');--> statement-breakpoint
CREATE TYPE "public"."goal_status" AS ENUM('active', 'paused', 'completed', 'archived');--> statement-breakpoint
CREATE TABLE "goal_contributions" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"goal_id" uuid NOT NULL,
	"amount_cents" integer NOT NULL,
	"contribution_date" date NOT NULL,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "goal_contributions_amount_positive" CHECK ("goal_contributions"."amount_cents" > 0)
);
--> statement-breakpoint
CREATE TABLE "goals" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"name" text NOT NULL,
	"target_amount_cents" integer NOT NULL,
	"current_amount_cents" integer DEFAULT 0 NOT NULL,
	"deadline" date,
	"priority" "goal_priority" DEFAULT 'medium' NOT NULL,
	"status" "goal_status" DEFAULT 'active' NOT NULL,
	"notes" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "goals_target_positive" CHECK ("goals"."target_amount_cents" > 0),
	CONSTRAINT "goals_current_not_negative" CHECK ("goals"."current_amount_cents" >= 0)
);
--> statement-breakpoint
ALTER TABLE "goal_contributions" ADD CONSTRAINT "goal_contributions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "goal_contributions" ADD CONSTRAINT "goal_contributions_goal_id_goals_id_fk" FOREIGN KEY ("goal_id") REFERENCES "public"."goals"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "goals" ADD CONSTRAINT "goals_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;