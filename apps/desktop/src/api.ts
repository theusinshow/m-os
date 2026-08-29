import { invoke } from "@tauri-apps/api/core";
import type { UndoStep } from "./hermes";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import type { EstadoDaAtualizacao } from "./atualizacao";
import type { Ocorrencia } from "./types";
import type { AnalysisConsent, InsightPreview, Meeting, MeetingAnalysis, MeetingInsight,
  MeetingTick, TranscriberStatus, TranscriptSegment,
  VoiceAction, VoiceNote, VoiceStopped, VoiceTick,
  WidgetPlacement, WidgetPlacementInput, RadialPin, RadialPinInput, Reminder, ReminderTarget, ActiveTimer, ActivityEvent, ActivityType, AppCapabilities, CalendarItem, Client, ClientInput, InvoiceData, Issuer, MonitoredApp, MonitoringSettings, PendingReminder, Period, ProjectTracking, ReportLine, ReportPdfData, SilencedApp, TrackingSettings, AppCatalogEntry, AppLaunchKind, AppStatus, BackupInspection, BackupReceipt, Capture, CaptureSource, DailyContext, DailySessionSummary, DailyToday, DropContext, EndDayInput, FunctionDefinition, Ingestion, IngestionReceipt, HiddenWidget, ImportReport, ObjectiveDraft, ObjectivePriority, ObjectiveStatus, Project, RegisteredApp, TimeEntry, Resource, ResourceKind, ResourceWorkspace, SearchItem, StartDayInput, AcademicDashboard, AcademicToday, Assignment, AssignmentStatus, Exam, ExamStatus, ReminderPriority, Semester, StaleView, SyncRound, SyncStatus, StudySession, Subject, Task, TaskState, Week, WeekSummary, TimeEntryEdit, Totals, UpdateInfo, UpdateProgress, Workspace, UnivirtusStatus, SyncReport, ProviderSubjectFact, Decision, Faixa } from "./types";

/**
 * O `Update` que a ultima verificacao devolveu.
 *
 * Ele nao e so um objeto: cada `check()` abre um RECURSO do lado Rust, com um
 * `rid` na tabela de recursos da webview. Verificar dez vezes abria dez, e
 * nenhum era fechado — o `close()` do plugin nunca era chamado. Por isso
 * `guardarPendente` fecha o anterior antes de guardar o novo, e por isso o
 * `catch` do `checkForUpdate` limpa: um `Update` velho sobrevivendo a uma
 * verificacao que FALHOU e um botao "Atualizar agora" que instala o que a
 * verificacao de agora nao confirmou.
 */
let pendingUpdate: Update | null = null;

async function guardarPendente(proximo: Update | null) {
  // `close()` pode falhar se o recurso ja foi embora — e ai nao ha nada a
  // fazer, e nada a dizer. O que nao pode e derrubar a verificacao por causa da
  // faxina da anterior.
  if (pendingUpdate && pendingUpdate !== proximo) {
    await pendingUpdate.close().catch(() => undefined);
  }
  pendingUpdate = proximo;
}

/**
 * O recibo de uma aceitacao.
 *
 * Vive aqui, e nao em `types.ts`, porque carrega o `UndoStep` — que mora junto
 * da ponte do Hermes por ser o contrato do desfazer do M/OS inteiro. Manter os
 * dois no mesmo lado evita um import de tipo atravessando na direcao errada.
 */
export type AcceptReceipt = {
  insight: MeetingInsight;
  taskId: string;
  reminderId: string | null;
  undo: UndoStep;
};

/**
 * O recibo de uma fala entendida.
 *
 * Vive aqui, e nao em `types.ts`, pela mesma razao do `AcceptReceipt`: ele
 * carrega o `UndoStep`, que mora junto da ponte do Hermes por ser o contrato do
 * desfazer do M/OS inteiro.
 *
 * `executed` distingue as duas coisas que o mesmo objeto descreve: o que o M/OS
 * FEZ (confianca alta) e o que ele OFERECE fazer (confianca media). Sem essa
 * separacao o recibo mentiria numa das duas.
 */
export type VoiceResult = {
  noteId: string;
  captureId: string;
  transcript: string;
  title: string;
  action: VoiceAction;
  confidence: "high" | "medium" | "low";
  executed: boolean;
  taskId: string | null;
  reminderId: string | null;
  projectId: string | null;
  projectName: string;
  /** O Project veio da tela, e nao da fala. */
  projectFromContext: boolean;
  /** O prazo COMO FOI DITO, e o instante para o qual foi resolvido. */
  whenRaw: string;
  when: string | null;
  hedged: boolean;
  undo: UndoStep | null;
  receiptMs: number;
};

export const api = {
  widgetPlacements() {
    return invoke<WidgetPlacement[]>("widget_placements");
  },

  // ===========================================================================
  // Daily Session
  // ===========================================================================
  //
  // Nenhum destes carrega a data: quem decide que dia e hoje e o backend, que
  // le o fuso publicado por `surfaceSetLocale`. Mandar a data daqui daria dois
  // lugares para a mesma pergunta ser respondida — e o atalho global e o Hermes
  // disparam do lado do Rust, onde nao ha renderer para carrega-la junto.
  //
  // As mutacoes devolvem o dia INTEIRO, e nao so o que mudou: a tela precisa do
  // progresso e da ordem depois de cada gesto, e devolver so o objetivo mexido
  // obrigaria o front a recalcular os dois — que e a regra saindo do dominio.

  /** O dia de hoje: sessao, objetivos, reflexao e a sessao velha em aberto. */
  dailyToday() {
    return invoke<DailyToday>("daily_today");
  },
  /** O que o M/OS ja sabe sobre hoje, antes de a pessoa escolher. */
  dailyContext() {
    return invoke<DailyContext>("daily_context");
  },
  dailyHistory() {
    return invoke<DailySessionSummary[]>("daily_history");
  },
  /** Uma sessao passada, inteira. */
  dailySession(id: string) {
    return invoke<DailyToday>("daily_session", { id });
  },
  dailyStart(input: StartDayInput) {
    return invoke<DailyToday>("daily_start", { input });
  },
  dailyAddObjective(draft: ObjectiveDraft, priority: ObjectivePriority) {
    return invoke<DailyToday>("daily_add_objective", { draft, priority });
  },
  dailyUpdateObjective(id: string, title: string, description: string) {
    return invoke<DailyToday>("daily_update_objective", { id, title, description });
  },
  dailySetObjectiveStatus(id: string, status: ObjectiveStatus) {
    return invoke<DailyToday>("daily_set_objective_status", { id, status });
  },
  dailySetMain(id: string) {
    return invoke<DailyToday>("daily_set_main", { id });
  },
  dailyRemoveObjective(id: string) {
    return invoke<DailyToday>("daily_remove_objective", { id });
  },
  /** A lista INTEIRA, e nao um movimento: quem conhece a ordem e a tela. */
  dailyReorder(sessionId: string, order: string[]) {
    return invoke<DailyToday>("daily_reorder", { sessionId, order });
  },
  /** `sessionId` nulo encerra o dia de hoje; com id, encerra o que ficou aberto. */
  dailyEnd(input: EndDayInput, sessionId: string | null = null) {
    return invoke<DailyToday>("daily_end", { sessionId, input });
  },
  dailyReopen(sessionId: string) {
    return invoke<DailyToday>("daily_reopen", { sessionId });
  },

  // ------------------------------------------------------------ Weekly Review
  //
  // `weeklyWeek()` sem argumento devolve a semana CORRENTE: quem decide que
  // semana e hoje e o backend, que le o fuso publicado por `surfaceSetLocale` —
  // mesmo motivo dos comandos do dia.

  weeklyWeek(week?: Week) {
    return invoke<WeekSummary>("weekly_week", { week: week ?? null });
  },
  /** A semana que acabou e nao foi fechada, ou `null`. */
  weeklyPending() {
    return invoke<Week | null>("weekly_pending");
  },
  weeklyClose(week: Week, summary: string) {
    return invoke<WeekSummary>("weekly_close", { week, summary });
  },

  // =========================================================================
  // M/Academic
  // =========================================================================
  //
  // O painel e o "hoje" vem COMPOSTOS do Rust: quem decide o que e "chegando",
  // como a media pondera e o que e atraso e `mos_core::academic`, e refazer
  // isso aqui daria duas respostas para a mesma pergunta.

  academicDashboard() {
    return invoke<AcademicDashboard>("academic_dashboard");
  },
  academicToday() {
    return invoke<AcademicToday>("academic_today");
  },

  // ---- Semestre
  academicSemesters(includeArchived = false) {
    return invoke<Semester[]>("academic_semesters", { includeArchived });
  },
  academicCreateSemester(name: string, institution: string, startsOn: string, endsOn: string) {
    return invoke<Semester>("academic_create_semester", { name, institution, startsOn, endsOn });
  },
  academicUpdateSemester(id: string, name: string, institution: string, startsOn: string, endsOn: string) {
    return invoke<Semester>("academic_update_semester", { id, name, institution, startsOn, endsOn });
  },
  academicArchiveSemester(id: string, archived: boolean) {
    return invoke<Semester>("academic_archive_semester", { id, archived });
  },

  // ---- Disciplina
  academicSubjects(includeArchived = false) {
    return invoke<Subject[]>("academic_subjects", { includeArchived });
  },
  academicCreateSubject(semesterId: string, name: string, code: string, teacher: string, accent: string, notes: string) {
    return invoke<Subject>("academic_create_subject", { semesterId, name, code, teacher, accent, notes });
  },
  academicUpdateSubject(id: string, name: string, code: string, teacher: string, accent: string, notes: string) {
    return invoke<Subject>("academic_update_subject", { id, name, code, teacher, accent, notes });
  },
  academicArchiveSubject(id: string, archived: boolean) {
    return invoke<Subject>("academic_archive_subject", { id, archived });
  },

  // ---- Atividade
  academicAssignments(includeArchived = false) {
    return invoke<Assignment[]>("academic_assignments", { includeArchived });
  },
  academicCreateAssignment(input: {
    subjectId: string;
    title: string;
    description: string;
    dueAt: string | null;
    priority: ReminderPriority;
    weight: number;
    score: number | null;
    maxScore: number | null;
  }) {
    return invoke<Assignment>("academic_create_assignment", input);
  },
  academicUpdateAssignment(input: {
    id: string;
    title: string;
    description: string;
    dueAt: string | null;
    priority: ReminderPriority;
    weight: number;
    score: number | null;
    maxScore: number | null;
    status: AssignmentStatus;
  }) {
    return invoke<Assignment>("academic_update_assignment", input);
  },
  academicSetAssignmentStatus(id: string, status: AssignmentStatus) {
    return invoke<Assignment>("academic_set_assignment_status", { id, status });
  },
  academicArchiveAssignment(id: string, archived: boolean) {
    return invoke<Assignment>("academic_archive_assignment", { id, archived });
  },
  /** Cria a Task do M/OS que executa a atividade, e liga as duas. */
  academicCreateTask(id: string) {
    return invoke<Task>("academic_create_task", { id });
  },
  academicUnlinkTask(id: string) {
    return invoke<Assignment>("academic_unlink_task", { id });
  },

  // ---- Avaliacao
  academicExams(includeArchived = false) {
    return invoke<Exam[]>("academic_exams", { includeArchived });
  },
  academicCreateExam(input: {
    subjectId: string;
    name: string;
    at: string;
    location: string;
    topics: string;
    weight: number;
    score: number | null;
    maxScore: number | null;
  }) {
    return invoke<Exam>("academic_create_exam", input);
  },
  academicUpdateExam(input: {
    id: string;
    name: string;
    at: string;
    location: string;
    topics: string;
    weight: number;
    score: number | null;
    maxScore: number | null;
    status: ExamStatus;
  }) {
    return invoke<Exam>("academic_update_exam", input);
  },
  academicArchiveExam(id: string, archived: boolean) {
    return invoke<Exam>("academic_archive_exam", { id, archived });
  },

  // ---- A decisão da pessoa
  //
  // Separadas das de `status`: aquelas descrevem o fato acadêmico e o sync do
  // Univirtus as escreve; estas são a decisão de quem estuda, e nenhum provedor
  // externo as toca.

  academicSetAssignmentDecision(id: string, decision: Decision) {
    return invoke<Assignment>("academic_set_assignment_decision", { id, decision });
  },
  academicSetExamDecision(id: string, decision: Decision) {
    return invoke<Exam>("academic_set_exam_decision", { id, decision });
  },
  /** `plannedAt` nulo desfaz o plano. */
  academicPlanAssignment(id: string, plannedAt: string | null, minutes: number) {
    return invoke<Assignment>("academic_plan_assignment", { id, plannedAt, minutes });
  },
  academicPlanExam(id: string, plannedAt: string | null, minutes: number) {
    return invoke<Exam>("academic_plan_exam", { id, plannedAt, minutes });
  },

  // ---- Materiais
  academicMaterials(subjectId: string) {
    return invoke<Resource[]>("academic_materials", { subjectId });
  },
  academicLinkMaterial(subjectId: string, resourceId: string, linked: boolean) {
    return invoke<void>("academic_link_material", { subjectId, resourceId, linked });
  },

  // ---- Univirtus
  //
  // Nenhuma destas funcoes conhece um endpoint do portal. O que atravessa e o
  // vocabulario do M/OS; o dialeto do AVA morre em `crate::univirtus`, atras da
  // allowlist.

  univirtusStatus() {
    return invoke<UnivirtusStatus>("univirtus_status");
  },
  /** Abre a pagina oficial numa janela e espera o login. Nao pede senha. */
  univirtusConnect() {
    return invoke<UnivirtusStatus>("univirtus_connect");
  },
  univirtusSync() {
    return invoke<SyncReport>("univirtus_sync");
  },
  univirtusDisconnect() {
    return invoke<void>("univirtus_disconnect");
  },
  univirtusSubjectFacts() {
    return invoke<ProviderSubjectFact[]>("univirtus_subject_facts");
  },
  /** O ultimo endereco visto de um material. Pode estar vencido: e cache. */
  univirtusMaterialUrl(externalId: string) {
    return invoke<string | null>("univirtus_material_url", { externalId });
  },

  // ---- Estudo
  academicStudySessions(limit = 50) {
    return invoke<StudySession[]>("academic_study_sessions", { limit });
  },
  academicStartStudy(subjectId: string, topic: string) {
    return invoke<StudySession>("academic_start_study", { subjectId, topic });
  },
  academicFinishStudy(id: string, seconds: number, notes: string) {
    return invoke<StudySession>("academic_finish_study", { id, seconds, notes });
  },
  academicDiscardStudy(id: string) {
    return invoke<void>("academic_discard_study", { id });
  },

  // ---------------------------------------------------------------- Paradas
  //
  // Sem argumento: obsolescencia e sempre "ate agora", e um parametro de data so
  // ofereceria uma pergunta que ninguem faz.

  staleList() {
    return invoke<StaleView>("stale_list");
  },

  // ===========================================================================
  // Voice Inbox
  // ===========================================================================
  //
  // Nenhum destes carrega audio: a captura inteira vive no Rust, e o que sobe
  // para ca e estado. `voiceRecording` le atomicos, entao chamar a cada quadro
  // custa quase nada.
  //
  // `voiceStart` e `voiceStop` sao chamados TAMBEM pelo atalho global, do lado
  // do Rust. Por isso o contexto e o fuso nao viajam nestes comandos: eles sao
  // publicados antes, por `surfaceSetContext` e `surfaceSetLocale` — do outro
  // caminho nao ha renderer para carrega-los junto.
  voiceStart() {
    return invoke<VoiceNote>("voice_start");
  },
  voiceStop() {
    return invoke<VoiceStopped>("voice_stop");
  },
  /** Esc: para, joga o audio fora e apaga a linha. */
  voiceCancel() {
    return invoke<void>("voice_cancel");
  },
  /** `null` quando nao ha gravacao em curso. */
  voiceRecording() {
    return invoke<VoiceTick | null>("voice_recording");
  },
  /** As notas cujo audio ainda guarda o que o banco nao tem em texto. */
  voicePending() {
    return invoke<VoiceNote[]>("voice_pending");
  },
  voiceRetry(id: string) {
    return invoke<VoiceNote>("voice_retry", { id });
  },
  voiceDiscard(id: string) {
    return invoke<void>("voice_discard", { id });
  },
  /** Aceita a oferta que a confianca media deixou na tela. */
  voiceAct(noteId: string) {
    return invoke<VoiceResult>("voice_act", { noteId });
  },

  // ===========================================================================
  // Contexto ambiente
  // ===========================================================================
  //
  // O que a tela esta mostrando, e em que fuso. Lido pela voz E pelo Hermes:
  // "me lembra disso sexta as 15h" tem dois buracos, e a tela preenche os dois
  // — "disso" e o que esta aberto, e "sexta as 15h" so resolve contra o fuso.
  //
  // O backend guarda em vez de receber no comando porque nem todo caminho vem
  // do renderer: o atalho global da voz dispara do lado do Rust.

  /**
   * O que a tela esta mostrando agora.
   *
   * Para a voz, o Project e a Task sao SINAL e nao verdade: eles so entram
   * quando a fala nao citou Project nenhum. Para o Hermes valem o mesmo — sao
   * o que faz "me lembra disso" ter um "disso".
   */
  surfaceSetContext(input: {
    screen: string;
    projectId?: string | null;
    projectLabel?: string | null;
    taskId?: string | null;
    taskLabel?: string | null;
    workspaceId?: string | null;
    workspaceLabel?: string | null;
  }) {
    return invoke<void>("surface_set_context", { input });
  },

  /**
   * O fuso de quem esta na frente do computador.
   *
   * Quem conhece o fuso e a tela, e o banco guarda UTC — `CORE-FOUNDATION.md`
   * §5, e o mesmo padrao do `ReminderComposer`. Sem isto, "amanha as nove"
   * seria resolvido contra UTC e cairia no dia errado a cada virada de noite.
   */
  surfaceSetLocale() {
    return invoke<void>("surface_set_locale", { offsetMinutes: -new Date().getTimezoneOffset() });
  },

  // ===========================================================================
  // Meeting Agent
  // ===========================================================================
  //
  // Nenhum destes carrega audio: a captura inteira vive no Rust, e o que sobe
  // para ca e estado. `meetingRecording` le atomicos, entao chamar de segundo em
  // segundo custa quase nada.
  meetingStart(title: string, projectId?: string | null) {
    return invoke<Meeting>("meeting_start", { title, projectId: projectId ?? null });
  },
  meetingStop() {
    return invoke<Meeting>("meeting_stop");
  },
  // Pausar para o áudio ANTES de mudar o estado; retomar faz o inverso. Nos dois
  // casos o intervalo entre as duas coisas não pode gravar frame numa reunião
  // que a tela já mostra parada, nem o contrário.
  meetingPause() {
    return invoke<Meeting>("meeting_pause");
  },
  meetingResume() {
    return invoke<Meeting>("meeting_resume");
  },
  /** `null` quando nao ha gravacao em curso. */
  meetingRecording() {
    return invoke<MeetingTick | null>("meeting_recording");
  },
  meetings(includeArchived = false) {
    return invoke<Meeting[]>("meeting_list", { includeArchived });
  },
  meeting(id: string) {
    return invoke<Meeting>("meeting_get", { id });
  },
  meetingTranscript(id: string) {
    return invoke<TranscriptSegment[]>("meeting_transcript", { id });
  },
  meetingAnalysis(id: string) {
    return invoke<MeetingAnalysis | null>("meeting_analysis", { id });
  },
  meetingInsights(id: string) {
    return invoke<MeetingInsight[]>("meeting_insights", { id });
  },
  meetingPreviews(id: string) {
    return invoke<InsightPreview[]>("meeting_previews", { id });
  },
  meetingSetProject(id: string, projectId: string | null) {
    return invoke<Meeting>("meeting_set_project", { id, projectId });
  },
  meetingSetTitle(id: string, title: string) {
    return invoke<Meeting>("meeting_set_title", { id, title });
  },
  // Autosave: a tela chama com debounce. Sem botão de salvar, porque um botão de
  // salvar numa nota de reunião é uma chance de perder o que se escreveu.
  meetingSetNotes(id: string, notes: string) {
    return invoke<Meeting>("meeting_set_notes", { id, notes });
  },
  meetingSetArchived(id: string, archived: boolean) {
    return invoke<Meeting>("meeting_set_archived", { id, archived });
  },
  /** As reunioes que a abertura encontrou interrompidas e que esperam decisao. */
  meetingInterrupted() {
    return invoke<Meeting[]>("meeting_interrupted");
  },
  meetingProcessRecovered(id: string) {
    return invoke<Meeting>("meeting_process_recovered", { id });
  },
  /** Descarta a gravacao. Apaga o audio DEPOIS de mudar o estado. */
  meetingDiscard(id: string) {
    return invoke<Meeting>("meeting_discard", { id });
  },
  /**
   * Apaga a reuniao inteira: banco e audio, sem volta.
   *
   * NAO e o mesmo que `meetingDiscard`, e a diferenca e a que importa na tela:
   * descartar preserva a REUNIAO e apaga so o audio, para a memoria do que
   * aconteceu continuar existindo. Isto aqui e para a reuniao que nunca deveria
   * ter existido — gravacao aberta por engano, teste de microfone, deteccao que
   * pegou o filme errado.
   *
   * Por nao ter volta, ela nao passa pelo recibo do M/OS (ADR-035): quem chama
   * confirma antes.
   */
  meetingDelete(id: string) {
    return invoke<void>("meeting_delete", { id });
  },
  meetingTranscribe(id: string) {
    return invoke<Meeting>("meeting_transcribe", { id });
  },
  meetingAnalyze(id: string) {
    return invoke<Meeting>("meeting_analyze", { id });
  },
  meetingRetry(id: string) {
    return invoke<Meeting>("meeting_retry", { id });
  },
  meetingOpenCommitments() {
    return invoke<MeetingInsight[]>("meeting_open_commitments");
  },
  /**
   * Aceita um item: cria Task e, quando `remindAt` vier, Reminder.
   *
   * `remindAt` sai DAQUI resolvido para instante. O `dueHint` guarda a palavra
   * dita ("amanha"); quem sabe que horas isso significa e o fuso de quem esta
   * olhando, e esse fuso so existe no renderer.
   */
  meetingAcceptInsight(input: {
    insightId: string;
    title: string;
    description?: string;
    projectId?: string | null;
    remindAt?: Date | null;
  }) {
    return invoke<AcceptReceipt>("meeting_accept_insight", {
      insightId: input.insightId,
      title: input.title,
      description: input.description ?? null,
      projectId: input.projectId ?? null,
      remindAt: input.remindAt ? input.remindAt.toISOString() : null,
    });
  },
  meetingDismissInsight(insightId: string) {
    return invoke<MeetingInsight>("meeting_dismiss_insight", { insightId });
  },
  meetingTranscriberStatus() {
    return invoke<TranscriberStatus>("meeting_transcriber_status");
  },
  meetingSetTranscriber(binary: string, model: string, threads: number, vadModel: string) {
    return invoke<TranscriberStatus>("meeting_set_transcriber", { binary, model, threads, vadModel });
  },
  meetingAnalysisConsent() {
    return invoke<AnalysisConsent>("meeting_analysis_consent");
  },
  // --- A oferta de gravar (ADR-047) ---
  //
  // A janelinha some por `hide` e não `close`: ela sobrevive entre ofertas, como
  // a do lembrete. Recriar custaria justamente o tempo em que ela precisa
  // aparecer.
  fecharReuniaoDetectada() {
    return invoke<void>("fechar_reuniao_detectada");
  },
  // Silencia o AVISO para aquele processo, e nunca a observação — mesmo critério
  // do "não lembrar hoje".
  silenciarDeteccao(processo: string) {
    return invoke<void>("silenciar_deteccao", { processo });
  },
  meetingSetAnalysisConsent(granted: boolean) {
    return invoke<AnalysisConsent>("meeting_set_analysis_consent", { granted });
  },
  // Manda a faixa inteira, e nao "o widget X foi para a posicao 3": quem
  // sabe o que acontece com quem estava la e o front, que conhece a faixa.
  // Mover entre faixas manda as DUAS na mesma chamada, porque as duas mudaram.
  // `workspaceId` nulo e a visao "Todos", que arruma a propria Home.
  setWidgetLayout(workspaceId: string | null, placements: WidgetPlacementInput[]) {
    return invoke<WidgetPlacement[]>("set_widget_layout", { workspaceId, placements });
  },
  // Volta ao desenho APAGANDO as linhas. Gravar o catalogo por cima
  // petrificaria o desenho de hoje, que e o oposto do que a inversao faz.
  resetWidgetLayout(workspaceId: string | null) {
    return invoke<WidgetPlacement[]>("reset_widget_layout", { workspaceId });
  },
  // --- O leque (migration 0021) ---
  //
  // Devolve TODAS as pétalas, de todos os escopos, pelo mesmo motivo de
  // `widgetPlacements`: são poucas linhas, e uma chamada só deixa a troca de
  // Workspace filtrar em memória em vez de ir ao core a cada clique.
  radialPins() {
    return invoke<RadialPin[]>("radial_pins");
  },
  // Uma pétala por chamada. `workspaceId` nulo é a visão "Todos".
  setRadialPin(workspaceId: string | null, pin: RadialPinInput) {
    return invoke<RadialPin[]>("set_radial_pin", { workspaceId, pin });
  },
  // Limpar APAGA a linha e devolve o slot ao desenho — não grava alvo vazio,
  // que petrificaria o padrão de hoje.
  clearRadialPin(workspaceId: string | null, slot: number) {
    return invoke<RadialPin[]>("clear_radial_pin", { workspaceId, slot });
  },
  // --- Inicializacao com o Windows (ADR-043) ---
  //
  // `autostartEnabled` pergunta ao SISTEMA e nao a uma configuracao nossa: o
  // `auto-launch` tambem escreve na chave que o Gerenciador de Tarefas usa, e o
  // usuario pode desligar por la sem nos avisar. Espelhar isso num booleano
  // faria a tela afirmar "ligado" sobre algo desligado.
  autostartEnabled() {
    return invoke<boolean>("autostart_enabled");
  },
  setAutostart(enabled: boolean) {
    return invoke<boolean>("autostart_set", { enabled });
  },
  // Esta e preferencia nossa: o Windows sabe iniciar o programa, nao com que
  // cara. Por isso ela pode viver em settings.json sem criar segunda verdade.
  startMinimized() {
    return invoke<boolean>("start_minimized");
  },
  setStartMinimized(value: boolean) {
    return invoke<boolean>("set_start_minimized", { value });
  },
  // --- Attention System ---
  //
  // `at` e `until` viajam como instante RFC 3339 ja resolvido. O calculo de
  // "amanha de manha" acontece AQUI de proposito: a interface e o unico lado
  // que conhece o fuso de quem clicou. Mesmo padrao que o lembrete do monitor
  // ja usa para o fim do dia.
  createReminder(title: string, body: string, at: Date, target?: ReminderTarget) {
    return invoke<Reminder>("attention_create", {
      input: {
        title,
        body,
        at: at.toISOString(),
        targetType: target?.type,
        targetId: target?.id,
      },
    });
  },
  reminders() {
    return invoke<Reminder[]>("attention_list");
  },
  attentionCount() {
    return invoke<number>("attention_count");
  },
  faixaDeUso() {
    return invoke<Faixa>("usage_faixa");
  },
  /** Alterna o painel da faixa. Devolve se ele ficou aberto. */
  alternarPainelDaFaixa() {
    return invoke<boolean>("faixa_painel_alternar");
  },
  fecharPainelDaFaixa() {
    return invoke<void>("faixa_painel_fechar");
  },
  /** Recolhe a faixa na lingueta, ou a traz de volta. Grava a preferência. */
  recolherFaixa(recolhida: boolean) {
    return invoke<void>("faixa_recolher", { recolhida });
  },
  /** Traz a janela principal para a frente. */
  abrirApp() {
    return invoke<void>("faixa_abrir_app");
  },
  snoozeReminder(id: string, until: Date) {
    return invoke<Reminder>("attention_snooze", { id, until: until.toISOString() });
  },
  completeReminder(id: string) {
    return invoke<Reminder>("attention_complete", { id });
  },
  acknowledgeReminder(id: string) {
    return invoke<Reminder>("attention_acknowledge", { id });
  },
  cancelReminder(id: string) {
    return invoke<Reminder>("attention_cancel", { id });
  },
  archiveReminder(id: string) {
    return invoke<Reminder>("attention_archive", { id });
  },
  // ===========================================================================
  // Universal Drop Zone
  // ===========================================================================
  //
  // Quatro chamadas para um arquivo, e a divisao nao e cerimonia: `begin` grava
  // a Capture ANTES do primeiro byte, `chunk` empurra os bytes crus, `finish`
  // preserva e so entao entende. `abort` existe para o caso em que a leitura no
  // renderer falha no meio — sem ele, uma transferencia morta ficaria aberta no
  // backend segurando um arquivo no staging.
  ingestBegin(descriptor: { name: string; mime: string; size: number; context: DropContext }) {
    return invoke<Ingestion>("ingest_begin", { descriptor });
  },
  /**
   * Um pedaco de arquivo, em bytes crus.
   *
   * O `ArrayBuffer` sozinho como argumento faz o Tauri usar o corpo bruto da
   * requisicao em vez de JSON — o id da ingestao viaja no header porque o corpo
   * ja esta ocupado sendo o arquivo.
   */
  ingestChunk(ingestionId: string, bytes: ArrayBuffer) {
    return invoke<void>("ingest_chunk", bytes, { headers: { "x-mos-ingestion": ingestionId } });
  },
  ingestFinish(ingestionId: string) {
    return invoke<IngestionReceipt>("ingest_finish", { id: ingestionId });
  },
  ingestAbort(ingestionId: string, reason: string) {
    return invoke<void>("ingest_abort", { id: ingestionId, reason });
  },
  ingestText(text: string, context: DropContext) {
    return invoke<IngestionReceipt>("ingest_text", { text, context });
  },
  ingestUrl(url: string, context: DropContext) {
    return invoke<IngestionReceipt>("ingest_url", { url, context });
  },
  ingestUndo(ingestionId: string) {
    return invoke<void>("ingest_undo", { id: ingestionId });
  },
  ingestAcceptSuggestion(ingestionId: string) {
    return invoke<void>("ingest_accept_suggestion", { id: ingestionId });
  },
  /** As ingestoes que viraram Resource, para a Library saber o que cada uma e. */
  ingestions() {
    return invoke<Ingestion[]>("list_ingestions");
  },
  openIngestedFile(resourceId: string) {
    return invoke<void>("open_ingested_file", { resourceId });
  },
  revealIngestedFile(resourceId: string) {
    return invoke<void>("reveal_ingested_file", { resourceId });
  },

  createCapture(content: string, source: CaptureSource) {
    return invoke<Capture>("create_capture", { input: { content, source } });
  },
  getCapture(id: string) {
    return invoke<Capture>("get_capture", { id });
  },
  recent() {
    return invoke<Capture[]>("list_recent");
  },
  inbox() {
    return invoke<Capture[]>("list_inbox");
  },
  archived() {
    return invoke<Capture[]>("list_archived");
  },
  trashed() {
    return invoke<Capture[]>("list_trashed");
  },
  search(query: string, includeArchived: boolean) {
    return invoke<SearchItem[]>("search_all", { query, includeArchived });
  },
  functions() {
    return invoke<FunctionDefinition[]>("list_functions");
  },
  searchFunctions(query: string) {
    return invoke<FunctionDefinition[]>("search_functions", { query });
  },
  markProcessed(id: string) {
    return invoke<Capture>("mark_capture_processed", { id });
  },
  moveToInbox(id: string) {
    return invoke<Capture>("move_capture_to_inbox", { id });
  },
  archive(id: string) {
    return invoke<Capture>("archive_capture", { id });
  },
  trash(id: string) {
    return invoke<Capture>("trash_capture", { id });
  },
  restore(id: string) {
    return invoke<Capture>("restore_capture", { id });
  },
  resources(includeArchived = false) {
    return invoke<Resource[]>("list_resources", { includeArchived });
  },
  trashedResources() {
    return invoke<Resource[]>("list_trashed_resources");
  },
  resource(id: string) {
    return invoke<Resource>("get_resource", { id });
  },
  searchResources(query: string, includeArchived = false) {
    return invoke<Resource[]>("search_resources", { query, includeArchived });
  },
  createResource(kind: ResourceKind, title: string, url: string, note: string, sourceCaptureId: string | null = null) {
    return invoke<Resource>("create_resource", { input: { kind, title, url, note, sourceCaptureId } });
  },
  updateResource(id: string, kind: ResourceKind, title: string, url: string, note: string) {
    return invoke<Resource>("update_resource", { input: { id, kind, title, url, note } });
  },
  setResourceArchived(id: string, archived: boolean) {
    return invoke<Resource>("set_resource_archived", { id, archived });
  },
  trashResource(id: string) {
    return invoke<Resource>("trash_resource", { id });
  },
  restoreResource(id: string) {
    return invoke<Resource>("restore_resource", { id });
  },
  openResource(id: string) {
    return invoke<void>("open_resource", { id });
  },
  /** Link vindo de uma resposta do Hermes. O backend recusa o que nao for http(s). */
  openExternalUrl(url: string) {
    return invoke<void>("open_external_url", { url });
  },
  deleteCapture(id: string) {
    return invoke<void>("delete_capture", { id });
  },
  deleteTask(id: string) {
    return invoke<void>("delete_task", { id });
  },
  deleteProject(id: string) {
    return invoke<void>("delete_project", { id });
  },
  deleteWorkspace(id: string) {
    return invoke<void>("delete_workspace", { id });
  },
  deleteRegisteredApp(id: string) {
    return invoke<void>("delete_registered_app", { id });
  },
  deleteResource(id: string) {
    return invoke<void>("delete_resource", { id });
  },
  resourceWorkspaces() {
    return invoke<ResourceWorkspace[]>("list_resource_workspaces");
  },
  setResourceWorkspace(resourceId: string, workspaceId: string, linked: boolean) {
    return invoke<void>("set_resource_workspace", { resourceId, workspaceId, linked });
  },
  projects(includeArchived = false) {
    return invoke<Project[]>("list_projects", { includeArchived });
  },
  createProject(name: string, description: string, repository = "") {
    return invoke<Project>("create_project", { input: { name, description, repository } });
  },
  updateProject(id: string, name: string, description: string, repository = "") {
    return invoke<Project>("update_project", { input: { id, name, description, repository } });
  },
  setProjectArchived(id: string, archived: boolean) {
    return invoke<Project>("set_project_archived", { id, archived });
  },
  tasks(includeArchived = false) {
    return invoke<Task[]>("list_tasks", { includeArchived });
  },
  createTask(title: string, description: string, projectId: string | null, sourceCaptureId: string | null = null) {
    return invoke<Task>("create_task", { input: { title, description, projectId, sourceCaptureId } });
  },
  updateTask(id: string, title: string, description: string, projectId: string | null) {
    return invoke<Task>("update_task", { input: { id, title, description, projectId } });
  },
  setTaskState(id: string, taskState: TaskState) {
    return invoke<Task>("set_task_state", { id, taskState });
  },
  setTaskArchived(id: string, archived: boolean) {
    return invoke<Task>("set_task_archived", { id, archived });
  },
  registeredApps(includeArchived = false) {
    return invoke<RegisteredApp[]>("list_registered_apps", { includeArchived });
  },
  createRegisteredApp(name: string, description: string, sourceUrl: string | null, launchKind: AppLaunchKind | null, launchTarget: string | null) {
    return invoke<RegisteredApp>("create_registered_app", { input: { name, description, sourceUrl, launchKind, launchTarget } });
  },
  updateRegisteredApp(id: string, name: string, description: string, sourceUrl: string | null, launchKind: AppLaunchKind | null, launchTarget: string | null, capabilities: AppCapabilities) {
    return invoke<RegisteredApp>("update_registered_app", { input: { id, name, description, sourceUrl, launchKind, launchTarget, ...capabilities } });
  },
  appCatalog() {
    return invoke<AppCatalogEntry[]>("list_app_catalog");
  },
  registerAppCatalog(ids: string[]) {
    return invoke<RegisteredApp[]>("register_app_catalog", { ids });
  },
  setRegisteredAppArchived(id: string, archived: boolean) {
    return invoke<RegisteredApp>("set_registered_app_archived", { id, archived });
  },
  workspaces(includeArchived = false) {
    return invoke<Workspace[]>("list_workspaces", { includeArchived });
  },
  createWorkspace(name: string, description: string) {
    return invoke<Workspace>("create_workspace", { input: { name, description } });
  },
  updateWorkspace(id: string, name: string, description: string) {
    return invoke<Workspace>("update_workspace", { input: { id, name, description } });
  },
  setWorkspaceArchived(id: string, archived: boolean) {
    return invoke<Workspace>("set_workspace_archived", { id, archived });
  },
  workspaceProjects(id: string, includeArchived = false) {
    return invoke<Project[]>("list_workspace_projects", { id, includeArchived });
  },
  workspaceApps(id: string, includeArchived = false) {
    return invoke<RegisteredApp[]>("list_workspace_apps", { id, includeArchived });
  },
  projectWorkspaces(id: string) {
    return invoke<Workspace[]>("list_project_workspaces", { id });
  },
  appWorkspaces(id: string) {
    return invoke<Workspace[]>("list_app_workspaces", { id });
  },
  setProjectWorkspace(projectId: string, workspaceId: string, linked: boolean) {
    return invoke<void>("set_project_workspace", { projectId, workspaceId, linked });
  },
  setAppWorkspace(appId: string, workspaceId: string, linked: boolean) {
    return invoke<void>("set_app_workspace", { appId, workspaceId, linked });
  },
  hiddenWidgets() {
    return invoke<HiddenWidget[]>("list_hidden_widgets");
  },
  // `workspaceId` nulo e a visao "Todos", que esconde os proprios widgets.
  setWorkspaceWidget(widgetId: string, workspaceId: string | null, visible: boolean) {
    return invoke<void>("set_workspace_widget", { widgetId, workspaceId, visible });
  },
  markRegisteredAppOpened(id: string) {
    return invoke<RegisteredApp>("mark_registered_app_opened", { id });
  },
  openRegisteredApp(id: string) {
    return invoke<RegisteredApp>("open_registered_app", { id });
  },
  rebuildSearch() {
    return invoke<number>("rebuild_search");
  },
  status() {
    return invoke<AppStatus>("get_app_status");
  },
  /** O atalho SEGURADO da voz. Recusa o mesmo atalho da Captura rapida. */
  setVoiceShortcut(shortcut: string) {
    return invoke<string>("set_voice_shortcut", { shortcut });
  },
  setShortcut(shortcut: string) {
    return invoke<string>("set_capture_shortcut", { shortcut });
  },
  showQuickCapture() {
    return invoke<void>("show_quick_capture");
  },
  hideQuickCapture() {
    return invoke<void>("hide_quick_capture");
  },
  /**
   * Tudo o que o M/OS registrou entre dois instantes.
   *
   * A janela vai como instante e não como data: quem decide onde um dia começa
   * é esta ponta, que conhece o fuso de quem está olhando.
   */
  calendarWindow(since: string, until: string) {
    return invoke<CalendarItem[]>("calendar_window", { since, until });
  },
  /** Totais por Project, com o arredondamento configurado já aplicado. */
  trackingTotals() {
    return invoke<Record<string, Totals>>("tracking_totals");
  },
  /** As sessões, em tempo REAL — sem arredondar e sem descontar inatividade. */
  trackingEntries(projectId?: string) {
    return invoke<TimeEntry[]>("tracking_entries", { projectId: projectId ?? null });
  },
  /** Lança tempo que o cronômetro não contou. Fica marcado como `manual`. */
  trackingRecord(input: { projectId: string; startedAt: string; durationSeconds: number; description: string; activityType: ActivityType; billable: boolean }) {
    return invoke<TimeEntry>("tracking_record", input);
  },
  trackingEdit(id: string, edit: TimeEntryEdit) {
    return invoke<TimeEntry>("tracking_edit", { id, edit });
  },
  trackingTrash(id: string) {
    return invoke<void>("tracking_trash", { id });
  },
  trackingRestore(id: string) {
    return invoke<void>("tracking_restore", { id });
  },
  /** A lixeira: o que foi removido continua no banco e volta por aqui. */
  trackingTrashed() {
    return invoke<TimeEntry[]>("tracking_trashed");
  },
  /**
   * As sessões de um período, cada uma com o que vale.
   *
   * Data ausente é "sem borda deste lado". O cálculo vem do backend porque é lá
   * que o arredondamento vive — repetir a conta aqui daria um total de tela
   * diferente do total da fatura, e ninguém saberia qual acreditar.
   */
  trackingReport(since: string | null, until: string | null) {
    return invoke<ReportLine[]>("tracking_report", { since, until });
  },
  trackingIssuer() {
    return invoke<Issuer>("tracking_issuer");
  },
  trackingSetIssuer(issuer: Issuer) {
    return invoke<Issuer>("tracking_set_issuer", { issuer });
  },
  /** `false` quando o usuário fecha o diálogo sem escolher onde salvar. */
  exportReportPdf(report: ReportPdfData, suggestedName: string) {
    return invoke<boolean>("tracking_export_report_pdf", { report, suggestedName });
  },
  exportInvoicePdf(invoice: InvoiceData, suggestedName: string) {
    return invoke<boolean>("tracking_export_invoice_pdf", { invoice, suggestedName });
  },
  exportCsv(contents: string, suggestedName: string) {
    return invoke<boolean>("tracking_export_csv", { contents, suggestedName });
  },
  trackingSettings() {
    return invoke<TrackingSettings>("tracking_settings");
  },
  trackingSetSettings(settings: TrackingSettings) {
    return invoke<TrackingSettings>("tracking_set_settings", { settings });
  },
  /** Os dados de cobrança de todo Project que tem algum. */
  projectTracking() {
    return invoke<ProjectTracking[]>("tracking_project_tracking");
  },
  /**
   * Grava valor/hora, código, cliente e meta.
   *
   * Vale do momento da gravação em diante: cada sessão já guarda a taxa que
   * valia quando o trabalho aconteceu, e reajustar aqui não reescreve o passado.
   */
  setProjectTracking(tracking: ProjectTracking) {
    return invoke<ProjectTracking>("tracking_set_project_tracking", { tracking });
  },
  clients(includeArchived = false) {
    return invoke<Client[]>("tracking_clients", { includeArchived });
  },
  /** Cria quando `id` vem vazio, atualiza quando vem preenchido. */
  saveClient(id: string | null, input: ClientInput) {
    return invoke<Client>("tracking_save_client", { id, input });
  },
  setClientArchived(id: string, archived: boolean) {
    return invoke<Client>("tracking_set_client_archived", { id, archived });
  },
  /** O lembrete que a janelinha deve mostrar, se houver. */
  reminderPending() {
    return invoke<PendingReminder | null>("reminder_pending");
  },
  /** Fecha sem decidir nada. "Agora não" é uma resposta legítima. */
  reminderDismiss() {
    return invoke<void>("reminder_dismiss");
  },
  /**
   * Silencia um programa até o instante dado.
   *
   * O instante é calculado aqui porque "hoje" acaba à meia-noite LOCAL, e o
   * backend só conhece UTC.
   */
  reminderSuppress(processName: string, until: string) {
    return invoke<void>("reminder_suppress", { processName, until });
  },
  /** Os programas silenciados agora — para a tela poder mostrar e desfazer. */
  reminderSilenced() {
    return invoke<SilencedApp[]>("reminder_silenced");
  },
  reminderUnsilence(processName: string) {
    return invoke<void>("reminder_unsilence", { processName });
  },
  monitoringSettings() {
    return invoke<MonitoringSettings>("monitoring_settings");
  },
  /** Vale na próxima passada do laço — desligar tem efeito em segundos. */
  monitoringSetSettings(settings: MonitoringSettings) {
    return invoke<MonitoringSettings>("monitoring_set_settings", { settings });
  },
  monitoredApps() {
    return invoke<MonitoredApp[]>("monitoring_apps");
  },
  saveMonitoredApp(entry: MonitoredApp) {
    return invoke<MonitoredApp>("monitoring_save_app", { entry });
  },
  deleteMonitoredApp(id: string) {
    return invoke<void>("monitoring_delete_app", { id });
  },
  /** A janela é obrigatória: a Linha do Tempo é sempre sobre um período. */
  activityEvents(since: string, until: string) {
    return invoke<ActivityEvent[]>("monitoring_events", { since, until });
  },
  /** Só os períodos SEM sessão registrada — o resto seria contar duas vezes. */
  monitoringTimeline(since: string, until: string) {
    return invoke<Period[]>("monitoring_timeline", { since, until });
  },
  /** Entra como `reconstructed`: proposta pelo sistema, aceita por você. */
  recordFromTimeline(projectId: string, since: string, until: string, activityType: ActivityType) {
    return invoke<TimeEntry>("tracking_record_from_timeline", { projectId, since, until, activityType });
  },
  markActivityProcessed(id: string) {
    return invoke<void>("monitoring_mark_processed", { id });
  },
  timerDiscard() {
    return invoke<void>("timer_discard");
  },
  timerCurrent() {
    return invoke<ActiveTimer | null>("timer_current");
  },
  timerStart(projectId: string, description: string, activityType: ActivityType) {
    return invoke<ActiveTimer>("timer_start", { projectId, description, activityType });
  },
  timerSetRunning(running: boolean) {
    return invoke<ActiveTimer>("timer_set_running", { running });
  },
  /** Encerra e devolve a sessão gravada. */
  timerStop() {
    return invoke<TimeEntry>("timer_stop");
  },
  /** Onde o CronoCAD guarda o banco, se ele estiver instalado. */
  defaultCronocadPath() {
    return invoke<string | null>("tracking_default_cronocad_path");
  },
  /** Quando o CronoCAD foi importado, se foi. Pergunta ao banco, não à sessão. */
  cronocadImportedAt() {
    return invoke<string | null>("tracking_cronocad_imported_at");
  },
  /** Caminho de mão única, roda uma vez. A origem é aberta só para leitura. */
  importCronocad(path: string) {
    return invoke<ImportReport>("tracking_import_cronocad", { path });
  },

  // ---------------------------------------------------------------- sync
  /** Onde o hub está, se há segredo e quanto espera para subir. */
  syncStatus() {
    return invoke<SyncStatus>("sync_status");
  },
  /** Vazio desliga a sincronização sem apagar o segredo nem a fila. */
  syncSetEndpoint(url: string) {
    return invoke<void>("sync_set_endpoint", { url });
  },
  /** Vai para o Credential Manager. Não há como ler de volta. */
  syncSetToken(token: string) {
    return invoke<void>("sync_set_token", { token });
  },
  syncClearToken() {
    return invoke<void>("sync_clear_token");
  },
  /** Uma rodada agora: empurra a fila, puxa o que mudou, reconcilia. */
  syncNow() {
    return invoke<SyncRound>("sync_now");
  },

  /**
   * A tela abriu. Libera a primeira rodada automatica.
   *
   * Sem isto o daemon espera 30s e roda assim mesmo. O sinal existe para a
   * primeira rodada nao segurar o banco durante a rajada de IPC do boot.
   */
  syncAppReady() {
    return invoke<void>("sync_app_pronto");
  },

  /** O resumo do dia foi lido. */
  syncDismissSummary() {
    return invoke<void>("sync_dispensar_resumo");
  },
  createBackup(path: string) {
    return invoke<BackupReceipt>("create_backup", { path });
  },
  inspectBackup(path: string) {
    return invoke<BackupInspection>("inspect_backup", { path });
  },
  restoreBackup(path: string) {
    return invoke<BackupReceipt>("restore_backup", { path });
  },
  exportJson(path: string) {
    return invoke<BackupReceipt>("export_json", { path });
  },
  /**
   * As ultimas ocorrencias do caderno local.
   *
   * Existe para a pessoa nao precisar abrir o `%APPDATA%` a mao quando algo
   * quebra. Um log que so o desenvolvedor alcanca e um log que so serve depois
   * que alguem ja perguntou — e as duas falhas que motivaram isto ("abre e
   * fecha sozinho", "a janelinha aparece com 404") acontecem no logon, longe de
   * qualquer terminal aberto.
   */
  diagnosticoRecente(limite = 40) {
    return invoke<Ocorrencia[]>("diagnostico_recente", { limite });
  },
  diagnosticoCaminho() {
    return invoke<string>("diagnostico_caminho");
  },
  async checkForUpdate() {
    let update: Update | null;
    try {
      update = await check({ timeout: 30_000 });
    } catch (causa) {
      // A verificacao caiu, entao o que sobrou da anterior nao vale mais. Sem
      // isto, o botao de instalar continuava vivo apontando para um resultado
      // que ninguem confirmou — e instalar o que a rede nao confirmou e pior
      // que nao instalar.
      await guardarPendente(null);
      throw causa;
    }
    await guardarPendente(update);
    if (!update) return null;
    return {
      currentVersion: update.currentVersion,
      version: update.version,
      date: update.date ?? null,
      body: update.body ?? "",
    } satisfies UpdateInfo;
  },
  /** O que o Rust guardou sobre atualizacao. Ver `atualizacao.rs`. */
  updateStatus() {
    return invoke<EstadoDaAtualizacao>("atualizacao_estado");
  },
  noteUpdateCheck(disponivel: string, publicadaEm: string) {
    return invoke<void>("atualizacao_anotar_verificacao", { disponivel, publicadaEm });
  },
  noteUpdateFailure(motivo: string) {
    return invoke<void>("atualizacao_anotar_falha", { motivo });
  },
  /**
   * Anota uma ocorrencia no caderno local.
   *
   * A atualizacao usa isto porque ela falha LONGE de qualquer terminal, e
   * "as vezes nao funciona" so vira defeito investigavel quando a falha deixa
   * uma linha com hora e motivo. Ver `diagnostico.rs`.
   */
  registrarOcorrencia(nivel: "info" | "aviso" | "erro", origem: string, mensagem: string) {
    return invoke<void>("diagnostico_registrar", { nivel, origem, mensagem });
  },
  /**
   * Baixa, instala e reinicia.
   *
   * O retorno distingue os dois finais possiveis, e a distincao importa: o
   * `relaunch` podia falhar DEPOIS de a atualizacao ja estar instalada, e o
   * `catch` de quem chama traduzia isso para "erro" — a tela dizia que a
   * atualizacao falhou quando ela tinha dado certo, e o proximo clique
   * esbarrava em "nenhuma atualizacao pendente". Instalado sem reiniciar nao e
   * um erro, e um passo que sobrou para o usuario.
   *
   * No Windows o instalador encerra o M/OS, entao o caminho comum e nem
   * retornar.
   */
  async installUpdate(onProgress: (progress: UpdateProgress) => void) {
    if (!pendingUpdate) throw new Error("Nenhuma atualizacao pendente.");
    let downloaded = 0;
    let total: number | null = null;
    await pendingUpdate.downloadAndInstall((event) => {
      if (event.event === "Started") {
        downloaded = 0;
        total = event.data.contentLength ?? null;
      }
      if (event.event === "Progress") downloaded += event.data.chunkLength;
      if (event.event === "Finished") downloaded = total ?? downloaded;
      onProgress({ downloaded, total });
    });
    await guardarPendente(null);
    try {
      await relaunch();
      return "reiniciando" as const;
    } catch {
      return "instalada" as const;
    }
  },
};

export function appError(error: unknown): { message: string; retryable: boolean } {
  if (error && typeof error === "object" && "message" in error) {
    const candidate = error as { message: unknown; retryable?: unknown };
    return {
      message: String(candidate.message),
      retryable: candidate.retryable === true,
    };
  }
  return { message: String(error), retryable: false };
}
