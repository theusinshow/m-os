-- Liga Row-Level Security em todas as tabelas do schema public.
-- Idempotente: pode rodar de novo. Não faz INSERT, UPDATE, DELETE nem TRUNCATE.
-- DROP POLICY só remove a definição da policy; as linhas continuam lá.
--
-- Dono:
--   public.users.supabase_user_id  = auth.uid()
--   demais tabelas.user_id         = public.users.id daquele auth.uid()
--
-- auth.uid() = user_id estaria errado: user_id é o id interno do app,
-- não o id do Supabase Auth. O servidor (DATABASE_URL, role postgres)
-- ignora RLS e segue gravando user_id como já faz hoje.
--
-- whatsapp_messages.user_id é anulável. Linha com user_id nulo não casa
-- com a policy e fica invisível para anon/authenticated. Só o role que
-- ignora RLS (postgres / service_role) lê essas linhas.
--
-- Tabela em public sem user_id e sem supabase_user_id: RLS ligado e
-- nenhuma policy. anon e authenticated não acessam. Não criar policy
-- USING (true).

DO $migration$
DECLARE
  tabela text;
  expressao text;
  comando text;
  expressao_usuario constant text := 'supabase_user_id = (SELECT auth.uid())';
  expressao_dono constant text :=
    'user_id = (SELECT id FROM public.users WHERE supabase_user_id = (SELECT auth.uid()))';
  tabelas_user_id constant text[] := ARRAY[
    'months',
    'incomes',
    'bill_categories',
    'recurrence_rules',
    'bills',
    'credit_cards',
    'credit_card_invoices',
    'credit_card_expenses',
    'pluggy_items',
    'subscriptions',
    'push_subscriptions',
    'whatsapp_messages',
    'whatsapp_pending_actions',
    'alerts',
    'monthly_snapshots',
    'purchase_simulations',
    'goals',
    'goal_contributions',
    'settings',
    'budgets',
    'financial_policies',
    'financial_insights',
    'financial_observer_runs',
    'mos_action_receipts'
  ];
  alvos text[];
BEGIN
  IF to_regprocedure('auth.uid()') IS NULL THEN
    RAISE EXCEPTION 'auth.uid() ausente. Rode esta migration no Postgres do Supabase.';
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
    RAISE EXCEPTION 'role authenticated ausente. Rode esta migration no Postgres do Supabase.';
  END IF;

  -- Qualquer tabela ordinária ou particionada em public, inclusive uma
  -- que o schema do app não conheça. ENABLE ROW LEVEL SECURITY é idempotente.
  FOR tabela IN
    SELECT c.relname
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname = 'public'
      AND c.relkind IN ('r', 'p')
    ORDER BY c.relname
  LOOP
    BEGIN
      EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', tabela);
    EXCEPTION
      WHEN insufficient_privilege THEN
        RAISE EXCEPTION 'sem permissao para ligar RLS em public.%', tabela;
    END;
  END LOOP;

  IF to_regclass('public.users') IS NULL THEN
    RAISE EXCEPTION 'tabela public.users ausente. Aplique as migrations anteriores antes.';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'users'
      AND column_name = 'supabase_user_id'
  ) THEN
    RAISE EXCEPTION 'public.users sem coluna supabase_user_id.';
  END IF;

  FOREACH tabela IN ARRAY tabelas_user_id LOOP
    IF to_regclass('public.' || tabela) IS NULL THEN
      RAISE EXCEPTION 'tabela public.% ausente. Aplique as migrations anteriores antes.', tabela;
    END IF;

    IF NOT EXISTS (
      SELECT 1
      FROM information_schema.columns
      WHERE table_schema = 'public'
        AND table_name = tabela
        AND column_name = 'user_id'
    ) THEN
      RAISE EXCEPTION 'public.% sem coluna user_id; RLS ficaria sem policy de dono.', tabela;
    END IF;
  END LOOP;

  alvos := ARRAY['users'] || tabelas_user_id;

  FOREACH tabela IN ARRAY alvos LOOP
    expressao := CASE WHEN tabela = 'users' THEN expressao_usuario ELSE expressao_dono END;

    FOREACH comando IN ARRAY ARRAY[
      'rls_select_own',
      'rls_insert_own',
      'rls_update_own',
      'rls_delete_own'
    ]
    LOOP
      IF EXISTS (
        SELECT 1
        FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = tabela
          AND policyname = comando
      ) THEN
        EXECUTE format('DROP POLICY %I ON public.%I', comando, tabela);
      END IF;
    END LOOP;

    EXECUTE format(
      'CREATE POLICY rls_select_own ON public.%I FOR SELECT TO authenticated USING (%s)',
      tabela, expressao);
    EXECUTE format(
      'CREATE POLICY rls_insert_own ON public.%I FOR INSERT TO authenticated WITH CHECK (%s)',
      tabela, expressao);
    EXECUTE format(
      'CREATE POLICY rls_update_own ON public.%I FOR UPDATE TO authenticated USING (%s) WITH CHECK (%s)',
      tabela, expressao, expressao);
    EXECUTE format(
      'CREATE POLICY rls_delete_own ON public.%I FOR DELETE TO authenticated USING (%s)',
      tabela, expressao);
  END LOOP;

  FOR tabela IN
    SELECT c.relname
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname = 'public'
      AND c.relkind IN ('r', 'p')
      AND NOT (c.relname = ANY (alvos))
    ORDER BY c.relname
  LOOP
    RAISE NOTICE
      'public.% sem coluna de dono conhecida: RLS ligado, sem policy. anon/authenticated nao acessam.',
      tabela;
  END LOOP;
END
$migration$;
