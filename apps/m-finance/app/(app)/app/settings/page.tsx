import { createCategory, setCategoryArchived, updateSettings } from "@/app/actions/settings";
import { PushToggle } from "@/components/push/push-toggle";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput } from "@/components/ui/validated-form";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getAllCategories, getSettingsForUser } from "@/lib/settings";

export default async function SettingsPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const settings = appUser ? await getSettingsForUser(appUser.id) : null;
  const categories = appUser ? await getAllCategories(appUser.id) : [];
  const alertDaysBefore = settings?.alertDaysBefore ?? 3;

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Configurações" title="Preferências" />

      <section className="space-y-3">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
            Rotina
          </p>
        </div>

        <div className="grid gap-4 lg:grid-cols-2">
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

          <DashboardCard title="Alertas">
            <ValidatedForm action={updateSettings} successMessage="Preferências salvas." className="space-y-4">
              <div>
                <label
                  className="mb-2 block text-sm font-medium text-text-secondary"
                  htmlFor="alert-days"
                >
                  Antecedência do lembrete
                </label>
                <ValidatedInput
                  className="field-input"
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
                  O app também alerta no dia do vencimento e quando algo fica vencido.
                </p>
              </div>
              <FormSubmitButton pendingLabel="Salvando...">Salvar preferências</FormSubmitButton>
            </ValidatedForm>
          </DashboardCard>
        </div>
      </section>

      <DashboardCard title="Notificações no celular">
        <PushToggle />
      </DashboardCard>

      <section className="space-y-3">
        <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
          Manutenção
        </p>

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
                  className="field-input"
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
      </section>
    </div>
  );
}
