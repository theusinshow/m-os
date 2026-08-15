import { createCategory, setCategoryArchived, updateSettings } from "@/app/actions/settings";
import { syncConnection, unlinkConnection } from "@/app/actions/openfinance";
import { ConnectBank } from "@/components/openfinance/connect-bank";
import { PushToggle } from "@/components/push/push-toggle";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput } from "@/components/ui/validated-form";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { getManagedCreditCards } from "@/lib/cards";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getPluggyConnections } from "@/lib/openfinance/items";
import { isPluggyConfigured } from "@/lib/env";
import { formatShortDate } from "@/lib/formatters/date";
import { getAllCategories, getSettingsForUser } from "@/lib/settings";

export default async function SettingsPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const settings = appUser ? await getSettingsForUser(appUser.id) : null;
  const categories = appUser ? await getAllCategories(appUser.id) : [];
  const cards = appUser ? await getManagedCreditCards(appUser.id) : [];
  const connections = appUser ? await getPluggyConnections(appUser.id) : [];
  const alertDaysBefore = settings?.alertDaysBefore ?? 3;
  const cardOptions = cards
    .filter((card) => card.isActive)
    .map((card) => ({ id: card.id, name: card.name }));

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Configurações" title="Base do app" />

      <section className="grid gap-4 lg:grid-cols-2">
        <DashboardCard title="Perfil">
          <div className="flex items-center gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-md border border-border-subtle bg-background-elevated text-lg font-semibold text-text-secondary">
              {user.email?.charAt(0).toUpperCase() ?? "M"}
            </div>
            <div className="min-w-0">
              <p className="truncate font-semibold text-text-primary">
                {user.user_metadata?.name ?? appUser?.name ?? "Matheus Mendes"}
              </p>
              <p className="truncate text-sm text-text-muted">{user.email}</p>
            </div>
          </div>
          <p className="mt-4 text-sm leading-6 text-text-muted">
            App privado. O acesso é restrito apenas à conta Google autorizada.
          </p>
        </DashboardCard>

        <DashboardCard title="Preferências de alerta">
          <ValidatedForm action={updateSettings} successMessage="Preferências salvas." className="space-y-4">
            <div>
              <label
                className="mb-2 block text-sm font-medium text-text-secondary"
                htmlFor="alert-days"
              >
                Avisar com quantos dias de antecedência
              </label>
              <ValidatedInput
                className="focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary"
                defaultValue={alertDaysBefore}
                id="alert-days"
                inputMode="numeric"
                max={30}
                min={0}
                name="alertDaysBefore"
                required
                type="number"
              />
              <p className="mt-2 text-xs leading-5 text-text-muted">
                Além deste aviso, o app sempre alerta no dia do vencimento e quando algo fica
                vencido.
              </p>
            </div>
            <FormSubmitButton pendingLabel="Salvando...">Salvar preferências</FormSubmitButton>
          </ValidatedForm>
        </DashboardCard>
      </section>

      <DashboardCard title="Categorias">
        <div className="grid gap-5 xl:grid-cols-[0.7fr_1fr]">
          <ValidatedForm action={createCategory} successMessage="Categoria adicionada." resetOnSuccess className="space-y-4">
            <div>
              <label
                className="mb-2 block text-sm font-medium text-text-secondary"
                htmlFor="category-name"
              >
                Nova categoria
              </label>
              <ValidatedInput
                className="focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted"
                id="category-name"
                name="name"
                placeholder="Pets"
                required
              />
            </div>
            <FormSubmitButton pendingLabel="Adicionando...">Adicionar categoria</FormSubmitButton>
          </ValidatedForm>

          <div className="space-y-2">
            {categories.length === 0 ? (
              <InlineEmpty>Nenhuma categoria cadastrada.</InlineEmpty>
            ) : (
              categories.map((category) => (
                <div
                  className="flex items-center justify-between gap-3 rounded-lg border border-border-subtle bg-background-elevated px-4 py-2.5"
                  key={category.id}
                >
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-text-primary">{category.name}</span>
                    {category.isArchived ? (
                      <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                        Arquivada
                      </span>
                    ) : null}
                  </div>
                  <ToastForm
                    action={setCategoryArchived}
                    successMessage={category.isArchived ? "Categoria restaurada." : "Categoria arquivada."}
                  >
                    <input name="categoryId" type="hidden" value={category.id} />
                    <input
                      name="isArchived"
                      type="hidden"
                      value={category.isArchived ? "false" : "true"}
                    />
                    <FormSubmitButton
                      pendingLabel={category.isArchived ? "Restaurando..." : "Arquivando..."}
                      variant="secondary"
                    >
                      {category.isArchived ? "Restaurar" : "Arquivar"}
                    </FormSubmitButton>
                  </ToastForm>
                </div>
              ))
            )}
          </div>
        </div>
      </DashboardCard>

      <DashboardCard title="Notificações no celular">
        <PushToggle />
      </DashboardCard>

      <DashboardCard title="Open Finance (Pluggy)">
        {!isPluggyConfigured() ? (
          <InlineEmpty>
            Defina PLUGGY_CLIENT_ID e PLUGGY_CLIENT_SECRET no .env para conectar bancos.
          </InlineEmpty>
        ) : (
          <div className="space-y-5">
            <p className="text-sm leading-6 text-text-muted">
              Conecte um banco para importar automaticamente as compras dos cartões de crédito. No
              sandbox você pode testar com bancos fictícios.
            </p>

            <ConnectBank cards={cardOptions} />

            {connections.length > 0 ? (
              <div className="space-y-2">
                {connections.map((connection) => (
                  <div
                    className="flex flex-wrap items-center gap-3 rounded-lg border border-border-subtle bg-background-elevated px-4 py-2.5"
                    key={connection.id}
                  >
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium text-text-primary">
                        {connection.connectorName ?? "Conta"}
                        {connection.cardName ? (
                          <span className="text-text-muted"> → {connection.cardName}</span>
                        ) : (
                          <span className="text-status-tight"> · sem cartão vinculado</span>
                        )}
                      </p>
                      <p className="text-xs text-text-muted">
                        {connection.status}
                        {connection.lastSyncedAt
                          ? ` · sincronizado em ${formatShortDate(connection.lastSyncedAt)}`
                          : " · nunca sincronizado"}
                      </p>
                    </div>
                    <ToastForm action={syncConnection} successMessage="Sincronização concluída.">
                      <input name="itemId" type="hidden" value={connection.itemId} />
                      <FormSubmitButton pendingLabel="Sincronizando..." variant="secondary">
                        Sincronizar agora
                      </FormSubmitButton>
                    </ToastForm>
                    <ToastForm action={unlinkConnection} successMessage="Conexão removida.">
                      <input name="itemId" type="hidden" value={connection.itemId} />
                      <FormSubmitButton pendingLabel="Removendo..." variant="danger">
                        Desvincular
                      </FormSubmitButton>
                    </ToastForm>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
        )}
      </DashboardCard>

      <DashboardCard title="Dados e backup">
        <p className="text-sm leading-6 text-text-muted">
          Exporte seus dados em CSV para abrir em planilha, ou baixe um backup completo em JSON.
        </p>
        <div className="mt-4 flex flex-wrap gap-2">
          <a
            className="focus-ring inline-flex min-h-11 items-center rounded-md border border-border-subtle px-4 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-default hover:bg-background-hover hover:text-text-primary"
            download
            href="/api/export/month"
          >
            CSV do mês
          </a>
          <a
            className="focus-ring inline-flex min-h-11 items-center rounded-md border border-border-subtle px-4 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-default hover:bg-background-hover hover:text-text-primary"
            download
            href="/api/export/history"
          >
            CSV do histórico
          </a>
          <a
            className="focus-ring inline-flex min-h-11 items-center rounded-md border border-border-subtle px-4 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-default hover:bg-background-hover hover:text-text-primary"
            download
            href="/api/export/backup"
          >
            Backup JSON
          </a>
        </div>
      </DashboardCard>

    </div>
  );
}
