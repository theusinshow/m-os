//! O executor das acoes do agente, sem janela (F4.1 do MOS-OVERHAUL-PLAN, ADR-080).
//!
//! Vivia em `apps/desktop/src-tauri/src/jarvis.rs`, preso ao `AppHandle` do
//! Tauri — e o bolso nao tinha como executar a mesma acao sem reescrever as
//! regras. Aqui ele recebe os `Servicos` e um `Capacidades` que o adapter
//! implementa: avisar a interface, acordar o agendador de lembretes. O que so
//! um aparelho sabe fazer (a ponte do M-Finance) fica no adapter dele, e o
//! executor responde "indisponivel neste aparelho" em vez de fingir.
//!
//! **A invariante continua a de sempre:** a acao do agente passa pelos MESMOS
//! servicos que a acao do usuario. Nunca SQL proprio, nunca um atalho
//! (ADR-024, ADR-032).

use time::OffsetDateTime;

use crate::{CoreError, Servicos};

/// O que o aparelho sabe fazer alem de gravar.
///
/// Os dois metodos tem padrao vazio: um servidor sem interface nao tem quem
/// avisar, e um aparelho sem agendador local nao tem quem acordar.
pub trait Capacidades: Send + Sync {
    /// Algo mudou: a interface recarrega o que mostra (`evento`, `tipo`).
    fn avisar(&self, _evento: &str, _tipo: &str) {}
    /// Um lembrete foi criado ou mudou: o agendador recalcula quando acordar.
    fn lembretes_mudaram(&self) {}
}

/// Um aparelho que nao avisa ninguem — o servidor, os testes.
pub struct SemCapacidades;
impl Capacidades for SemCapacidades {}

/// Tudo que uma execucao precisa saber.
pub struct Contexto<'a> {
    pub servicos: &'a Servicos,
    pub capacidades: &'a dyn Capacidades,
    /// Agora, em UTC. Instantes gravados.
    pub agora: OffsetDateTime,
    /// Agora, no fuso de QUEM PEDIU. Decide o que e "hoje" e como o recibo fala
    /// a hora.
    pub agora_local: OffsetDateTime,
}

fn indisponivel(mensagem: &str) -> CoreError {
    CoreError::new(crate::ErrorCode::InvalidInput, mensagem, false)
}

/// Executa uma acao aprovada. Ver o cabecalho do modulo.
pub fn executar_acao(
    ctx: &Contexto<'_>,
    args: &crate::ActionArgs,
) -> Result<crate::ActionEffect, CoreError> {
    let state = ctx.servicos;
    match args {
        crate::ActionArgs::CaptureCreate { content } => {
            let capture = state.captures.create(crate::CreateCaptureInput {
                content: content.clone(),
                source: crate::CaptureSource::Home,
            })?;
            Ok(crate::ActionEffect::new(
                format!("Capture criada: {}", capture.content),
                Some(crate::UndoStep::ArchiveCapture {
                    id: capture.id.to_string(),
                }),
            )
            .touching("capture", capture.id.to_string(), capture.content))
        }
        crate::ActionArgs::CaptureToTask {
            capture,
            title,
            project,
        } => {
            let origem = resolve_capture(state, capture)?;
            let project_id = match project {
                Some(name) => resolve_project(state, name)?,
                None => None,
            };
            // Titulo vazio significa "use o que esta escrito na Capture". O
            // dominio recusa Task sem titulo, e inventar um aqui seria pior:
            // a fala ja esta guardada na Capture, e copia-la e a leitura
            // honesta de "transforma isso em task".
            let titulo = if title.trim().is_empty() {
                origem.content.clone()
            } else {
                title.clone()
            };
            let task = state.work.create_task(crate::CreateTaskInput {
                title: titulo,
                description: String::new(),
                project_id,
                source_capture_id: Some(origem.id.to_string()),
                ..Default::default()
            })?;
            ctx.capacidades.avisar("capture-changed", "processed");
            Ok(crate::ActionEffect::new(
                format!("Capture virou a Task: {}", task.title),
                Some(crate::UndoStep::UndoCaptureToTask {
                    capture_id: origem.id.to_string(),
                    task_id: task.id.to_string(),
                }),
            )
            .touching("task", task.id.to_string(), task.title)
            .touching("capture", origem.id.to_string(), origem.content))
        }
        crate::ActionArgs::TaskSetProject { task, project } => {
            let alvo = resolve_task(state, task)?;
            let anterior = alvo.project_id.map(|id| id.to_string());
            let destino = resolve_project(state, project)?;
            // A Task inteira vai como esta, com o Project trocado: a escrita e
            // AUTORITATIVA campo por campo, e desde a 0039 sao onze campos. Um
            // `UpdateTaskInput` montado a mao aqui apagaria prazo, prioridade e
            // waiting-for ao mover a Task de Project — em silencio.
            let atualizada = state.work.update_task(crate::UpdateTaskInput {
                project_id: destino.clone(),
                ..crate::UpdateTaskInput::from_task(&alvo)
            })?;
            let nome = destino
                .as_deref()
                .and_then(|id| crate::ProjectId::parse(id).ok())
                .map(|id| project_name(state, id))
                .unwrap_or_else(|| "nenhum Project".to_owned());
            Ok(crate::ActionEffect::new(
                format!("{} agora e de {nome}.", atualizada.title),
                Some(crate::UndoStep::RestoreTaskProject {
                    id: atualizada.id.to_string(),
                    project_id: anterior,
                }),
            )
            .touching("task", atualizada.id.to_string(), atualizada.title))
        }
        crate::ActionArgs::ReminderCreate {
            title,
            body,
            at,
            target,
            persistent,
            waiting_for,
            ..
        } => {
            let instant = crate::parse_moment(at)?;
            let alvo = match target {
                Some(target) => Some(resolve_target(state, target)?),
                None => None,
            };
            // `ReminderSource::Hermes` existia no dominio desde o P0 e nunca
            // tinha sido escrito por ninguem. E ele que faz o lembrete saber de
            // onde veio, e e por ele que o Attention Score e a auditoria
            // distinguem o que a pessoa agendou do que o agente propos.
            //
            // Passa pelo `create` completo, e nao pelo `create_at`: e por ele
            // que "nao me deixa esquecer" vira um lembrete que INSISTE, e que
            // "cobrar o Victor" vira uma pergunta em vez de um "concluir".
            let reminder = state.attention.create(crate::CreateReminder {
                title: title.clone(),
                body: body.clone(),
                at: Some(instant),
                target: alvo.as_ref().map(|(target, _)| *target),
                priority: crate::Priority::Normal,
                source: crate::ReminderSource::Hermes,
                persistent: *persistent,
                recurrence: None,
                waiting_for: (!waiting_for.is_empty()).then(|| waiting_for.clone()),
                leads: Vec::new(),
            })?;
            // Sem isto o lembrete existe no banco e o agendador nao sabe: ele
            // so acordaria no proximo restart, e "me lembra em vinte minutos"
            // passaria em silencio.
            ctx.capacidades.lembretes_mudaram();
            let quando = crate::spoken_moment(instant.to_offset(ctx.agora_local.offset()));
            // A confirmacao diz que ele vai INSISTIR quando for o caso. Sem
            // isso, "nao me deixa esquecer" e "me lembra" produziriam a mesma
            // frase — e a pessoa nao teria como saber qual dos dois ela ganhou.
            let insistencia = if *persistent {
                " Vou continuar cobrando ate voce resolver."
            } else {
                ""
            };
            let mensagem = match &alvo {
                Some((_, rotulo)) => {
                    format!("Lembrete para {quando}, vinculado a \"{rotulo}\".{insistencia}")
                }
                None => format!("Lembrete para {quando}.{insistencia}"),
            };
            let mut effect = crate::ActionEffect::new(
                mensagem,
                Some(crate::UndoStep::CancelReminder {
                    id: reminder.id.to_string(),
                }),
            )
            .touching("reminder", reminder.id.to_string(), reminder.title.clone());
            if let Some((target, rotulo)) = alvo {
                let (kind, id) = target.as_columns();
                effect = effect.touching(kind, id, rotulo);
            }
            Ok(effect)
        }
        crate::ActionArgs::ReminderSnooze { reminder, at, .. } => {
            let instant = crate::parse_moment(at)?;
            let alvo = resolve_reminder(state, reminder)?;
            // `snooze` e nao `transition`: e o metodo do servico que registra o
            // adiamento no historico. Adiar quinze vezes e um sinal, e o sinal
            // so existe se cada adiamento for anotado.
            let adiado = state.attention.snooze(alvo.id, instant)?;
            ctx.capacidades.lembretes_mudaram();
            let quando = crate::spoken_moment(instant.to_offset(ctx.agora_local.offset()));
            Ok(crate::ActionEffect::new(
                format!("\"{}\" volta {quando}.", adiado.title),
                // Sem desfazer: o instante ANTERIOR ja nao esta em lugar nenhum
                // depois do adiamento, e um "desfazer" que devolvesse o lembrete
                // para uma hora inventada seria pior que nenhum. O caminho de
                // volta e adiar de novo, ou remarcar.
                None,
            )
            .touching("reminder", adiado.id.to_string(), adiado.title.clone()))
        }
        crate::ActionArgs::ReminderResolve {
            reminder,
            state: desfecho,
        } => {
            let alvo = resolve_reminder(state, reminder)?;
            let transition = if desfecho == "done" {
                crate::Transition::Complete
            } else {
                crate::Transition::Cancel
            };
            // Pelos METODOS do servico: e neles que a pilha de alertas
            // pendente e cancelada e que a proxima ocorrencia de uma serie
            // nasce. `transition` sozinho faria o Hermes concluir pela metade.
            let resolvido = if transition == crate::Transition::Complete {
                state.attention.complete(alvo.id)?
            } else {
                state.attention.cancel(alvo.id)?
            };
            ctx.capacidades.lembretes_mudaram();
            Ok(crate::ActionEffect::new(
                format!(
                    "Lembrete \"{}\" {}.",
                    resolvido.title,
                    if desfecho == "done" {
                        "concluido"
                    } else {
                        "cancelado"
                    }
                ),
                // SEM desfazer, e a ausencia e a decisao: concluir e cancelar
                // sao estados terminais no dominio (`attention.rs`), e nao ha
                // transicao de volta. Inventar uma aqui daria ao Hermes um
                // caminho que a tela nao tem.
                None,
            )
            .touching("reminder", resolvido.id.to_string(), resolvido.title))
        }
        crate::ActionArgs::TaskCreate {
            title,
            description,
            project,
            checklist,
            due,
            priority,
        } => {
            let project_id = match project {
                Some(name) => resolve_project(state, name)?,
                None => None,
            };
            // A Task e os passos nascem na MESMA transacao (`insert_task`), e e
            // por isso que o checklist vem no input em vez de virar N chamadas
            // aqui: uma queda no meio deixaria a Task no quadro com metade da
            // lista que a pessoa autorizou no cartao.
            let task = state.work.create_task(crate::CreateTaskInput {
                title: title.clone(),
                description: description.clone(),
                project_id,
                source_capture_id: None,
                due_at: (!due.is_empty()).then(|| due.clone()),
                priority: (!priority.is_empty()).then(|| priority.clone()),
                checklist: checklist.clone(),
                ..Default::default()
            })?;
            let passos = if task.checklist_total > 0 {
                format!(" ({} passos)", task.checklist_total)
            } else {
                String::new()
            };
            Ok(crate::ActionEffect::new(
                format!("Task criada: {}{passos}", task.title),
                Some(crate::UndoStep::ArchiveTask {
                    id: task.id.to_string(),
                }),
            )
            .touching("task", task.id.to_string(), task.title))
        }
        // Os passos entram na Task que JA EXISTE, e nunca numa nova. E o
        // motivo de a acao existir: sem ela, "adiciona enviar para o Victor no
        // checklist" so podia virar `mos.task.create`.
        crate::ActionArgs::TaskAddChecklist { task, items } => {
            let alvo = resolve_task(state, task)?;
            let antes: std::collections::HashSet<String> = state
                .work
                .task_detail(&alvo.id.to_string())?
                .checklist
                .iter()
                .map(|passo| passo.id.to_string())
                .collect();
            for passo in items {
                state.work.add_checklist_item(&alvo.id.to_string(), passo)?;
            }
            let depois = state.work.task_detail(&alvo.id.to_string())?;
            // O que o Undo apaga sao os ids que NAO existiam antes. Guardar o
            // retorno de cada criacao daria o mesmo resultado quando tudo da
            // certo, e um id perdido quando uma delas falha no meio.
            let criados: Vec<String> = depois
                .checklist
                .iter()
                .map(|passo| passo.id.to_string())
                .filter(|id| !antes.contains(id))
                .collect();
            Ok(crate::ActionEffect::new(
                format!(
                    "{} passo(s) em {}. Agora {}/{}.",
                    criados.len(),
                    alvo.title,
                    depois.task.checklist_done,
                    depois.task.checklist_total
                ),
                Some(crate::UndoStep::RemoveChecklistItems { ids: criados }),
            )
            .touching("task", alvo.id.to_string(), alvo.title))
        }
        crate::ActionArgs::TaskCheckItem { task, item, done } => {
            let alvo = resolve_task(state, task)?;
            let checklist = state.work.task_detail(&alvo.id.to_string())?.checklist;
            // O passo e casado pelo TEXTO dentro daquela Task: ninguem diz
            // "marca o 018f2a3c". `resolve` e o mesmo casador das outras
            // entidades — acerto forte e sozinho age, ambiguidade pergunta.
            let achado = crate::resolve(
                &checklist,
                item,
                |passo| passo.id.to_string(),
                |passo| passo.label.clone(),
            );
            if let Some(erro) = crate::resolution_error(
                &achado,
                crate::EntityKind::Task,
                item,
                |passo: &crate::ChecklistItem| passo.label.clone(),
            ) {
                return Err(erro);
            }
            let passo = achado.one().expect("sem erro ha exatamente um").clone();
            let antes = passo.completed();
            let atual = state
                .work
                .set_checklist_item_done(&passo.id.to_string(), *done)?;
            Ok(crate::ActionEffect::new(
                format!(
                    "{}: {} - {}/{}.",
                    if *done { "Concluido" } else { "Reaberto" },
                    passo.label,
                    atual.checklist_done,
                    atual.checklist_total
                ),
                Some(crate::UndoStep::RestoreChecklistItem {
                    id: passo.id.to_string(),
                    done: antes,
                }),
            )
            .touching("task", alvo.id.to_string(), alvo.title))
        }
        crate::ActionArgs::TaskSetPlan {
            task,
            due,
            clear_due,
            priority,
            estimate_minutes,
            waiting_for,
            follow_up,
        } => {
            let alvo = resolve_task(state, task)?;
            // A base e a Task INTEIRA como ela esta: a escrita e autoritativa
            // campo por campo, e montar o input a mao apagaria em silencio tudo
            // que a frase nao mencionou.
            let base = crate::UpdateTaskInput::from_task(&alvo);
            let anterior = base.clone();
            let atualizada = state.work.update_task(crate::UpdateTaskInput {
                due_at: if *clear_due {
                    None
                } else if due.is_empty() {
                    base.due_at.clone()
                } else {
                    Some(due.clone())
                },
                priority: if priority.is_empty() {
                    base.priority.clone()
                } else {
                    Some(priority.clone())
                },
                estimate_minutes: estimate_minutes.or(base.estimate_minutes),
                waiting_for: if waiting_for.is_empty() {
                    base.waiting_for.clone()
                } else {
                    waiting_for.clone()
                },
                follow_up_at: if follow_up.is_empty() {
                    base.follow_up_at.clone()
                } else {
                    Some(follow_up.clone())
                },
                ..base
            })?;
            Ok(crate::ActionEffect::new(
                format!("{} ajustada.", atualizada.title),
                Some(crate::UndoStep::RestoreTaskPlan {
                    id: alvo.id.to_string(),
                    due_at: anterior.due_at,
                    priority: anterior.priority.unwrap_or_default(),
                    estimate_minutes: anterior.estimate_minutes,
                    waiting_for: anterior.waiting_for,
                    follow_up_at: anterior.follow_up_at,
                }),
            )
            .touching("task", alvo.id.to_string(), alvo.title))
        }
        crate::ActionArgs::TaskSetState { task, state: next } => {
            // O estado anterior e lido ANTES da mudanca: depois nao ha de onde
            // tirar, e sem ele "mover" seria a unica acao sem caminho de volta.
            let target = resolve_task(state, task)?;
            let previous = target.state;
            let updated = state
                .work
                .set_task_state(&target.id.to_string(), crate::TaskState::parse(next)?)?;
            Ok(crate::ActionEffect::new(
                format!("{} movida para {}", updated.title, next),
                Some(crate::UndoStep::RestoreTaskState {
                    id: updated.id.to_string(),
                    state: previous.as_str().to_owned(),
                }),
            )
            .touching("task", updated.id.to_string(), updated.title))
        }
        crate::ActionArgs::TaskStart { task } => {
            let target = resolve_task(state, task)?;
            // O efeito INTEIRO volta do repositorio — inclusive quem perdeu o
            // foco —, e e isso que o recibo guarda para desfazer.
            let effect = state
                .work
                .start_task_recorded(&target.id.to_string(), ctx.agora)?;
            ctx.capacidades.avisar("data-changed", "task");
            let updated = effect.task.clone();
            Ok(crate::ActionEffect::new(
                format!("Começada: {}", updated.title),
                Some(crate::UndoStep::UndoTaskStart(
                    crate::UndoTaskStart::from_effect(&effect),
                )),
            )
            .touching("task", updated.id.to_string(), updated.title))
        }
        crate::ActionArgs::TaskPlan { task, day } => {
            let target = resolve_task(state, task)?;
            let dia = if day.is_empty() {
                None
            } else {
                Some(crate::Day::parse(day)?)
            };
            // Adiar conta como adiamento so quando ja havia um dia planejado
            // que ficou para tras — planejar pela primeira vez nao e evitar.
            let adiando = target.scheduled_for.is_some() && dia.is_some();
            let updated = state.work.plan_task(&target.id.to_string(), dia, adiando)?;
            ctx.capacidades.avisar("data-changed", "task");
            Ok(crate::ActionEffect::new(
                if day.is_empty() {
                    format!("{} saiu do planejamento.", updated.title)
                } else {
                    format!("{} planejada para {day}.", updated.title)
                },
                None,
            )
            .touching("task", updated.id.to_string(), updated.title))
        }
        crate::ActionArgs::ProjectCreate { name, description } => {
            let project = state.work.create_project(crate::CreateProjectInput {
                name: name.clone(),
                description: description.clone(),
                repository: String::new(),
            })?;
            Ok(crate::ActionEffect::new(
                format!("Project criado: {}", project.name),
                Some(crate::UndoStep::ArchiveProject {
                    id: project.id.to_string(),
                }),
            )
            .touching("project", project.id.to_string(), project.name))
        }
        crate::ActionArgs::ResourceCreate {
            kind,
            title,
            url,
            note,
        } => {
            let resource = state.memory.create_resource(crate::CreateResourceInput {
                kind: crate::ResourceKind::parse(kind)?,
                title: title.clone(),
                url: url.clone(),
                note: note.clone(),
                source_capture_id: None,
            })?;
            Ok(crate::ActionEffect::new(
                format!("Resource salvo: {}", resource.title),
                Some(crate::UndoStep::ArchiveResource {
                    id: resource.id.to_string(),
                }),
            )
            .touching("resource", resource.id.to_string(), resource.title))
        }
        crate::ActionArgs::TimeStart {
            project,
            activity,
            description,
        } => {
            let project_id = resolve_project_id(state, project)?;
            let started = state.tracking.start_timer(crate::StartTimer {
                project_id,
                description: description.clone(),
                activity_type: parse_activity(activity)?,
            })?;
            ctx.capacidades.avisar("timer-changed", "started");
            Ok(crate::ActionEffect::new(
                format!(
                    "Cronometro iniciado em {}.",
                    project_name(state, started.project_id)
                ),
                // SEM desfazer, e a ausencia e a decisao certa: o inverso de
                // iniciar e descartar, e descartar joga tempo fora. A ADR-035
                // diz que desfazer arquiva e nunca destroi, e um cronometro
                // recem-iniciado se resolve com Pausar ou Encerrar na tela.
                None,
            )
            .touching(
                "project",
                started.project_id.to_string(),
                project_name(state, started.project_id),
            ))
        }
        crate::ActionArgs::TimeStop => {
            let entry = state.tracking.stop_timer()?;
            ctx.capacidades.avisar("timer-changed", "stopped");
            Ok(crate::ActionEffect::new(
                format!(
                    "Sessao de {} gravada em {}.",
                    spoken_duration(entry.duration_seconds),
                    project_name(state, entry.project_id)
                ),
                Some(crate::UndoStep::TrashTimeEntry {
                    id: entry.id.to_string(),
                }),
            )
            .touching(
                "timeEntry",
                entry.id.to_string(),
                project_name(state, entry.project_id),
            ))
        }
        crate::ActionArgs::TimeRecord {
            project,
            minutes,
            day,
            activity,
            description,
        } => {
            let project_id = resolve_project_id(state, project)?;
            // A taxa vem do Project AGORA e vira snapshot, igual ao lancamento
            // feito pela tela: e a melhor aproximacao para uma hora lembrada
            // depois, e usar outro caminho aqui daria valores diferentes para a
            // mesma hora conforme quem a lancou.
            let rate = state.tracking.hourly_rate_for(project_id)?;
            let entry = state.tracking.record(crate::NewTimeEntry {
                project_id,
                started_at: moment_for_day(day)?,
                ended_at: None,
                duration_seconds: minutes * 60,
                idle_seconds: 0,
                description: description.clone(),
                activity_type: parse_activity(activity)?,
                billable: true,
                hourly_rate_snapshot_cents: rate,
                // `Manual` como qualquer hora lembrada depois. Hora proposta
                // pelo Hermes nao e mais medida que hora digitada na tela.
                source: crate::EntrySource::Manual,
            })?;
            ctx.capacidades.avisar("data-changed", "tracking");
            Ok(crate::ActionEffect::new(
                format!(
                    "{} lancadas em {}.",
                    spoken_duration(entry.duration_seconds),
                    project_name(state, entry.project_id)
                ),
                Some(crate::UndoStep::TrashTimeEntry {
                    id: entry.id.to_string(),
                }),
            )
            .touching(
                "timeEntry",
                entry.id.to_string(),
                project_name(state, entry.project_id),
            ))
        }
        // ------------------------------------------------------ Daily Session
        //
        // As cinco passam pelos MESMOS servicos que a interface usa
        // (`crate::daily`), e nao por SQL proprio. E a invariante da ADR-024: a
        // acao do Hermes obedece as mesmas regras que a acao do usuario, e nao
        // ha um segundo caminho que pudesse divergir.
        crate::ActionArgs::DayStart {
            main,
            main_ref,
            secondaries,
            note,
        } => {
            // O vinculo do principal e resolvido AQUI, e nao no dominio: so
            // este lado conhece o banco. Sem ele, um dia montado pelo Hermes
            // teria objetivos de texto solto e a conclusao automatica nunca
            // dispararia.
            let (link_kind, link_id) = match main_ref.trim() {
                "" => (String::new(), String::new()),
                referencia => match resolve_task(state, referencia) {
                    Ok(task) => ("task".to_owned(), task.id.to_string()),
                    // Task nao achada NAO e erro: a referencia pode ser um
                    // Project. So depois de os dois falharem e que a proposta
                    // cai — e ai a mensagem vem do Project, que e o tipo mais
                    // amplo e o palpite mais provavel de quem escreveu.
                    Err(_) => {
                        let id = resolve_project(state, referencia)?.ok_or_else(|| {
                            CoreError::new(
                                crate::ErrorCode::NotFound,
                                format!("Nao achei Task nem Project para \"{referencia}\"."),
                                false,
                            )
                        })?;
                        ("project".to_owned(), id)
                    }
                },
            };

            let input = crate::StartDayInput {
                main: Some(crate::ObjectiveDraft {
                    title: main.clone(),
                    link_kind,
                    link_id,
                    ..Default::default()
                }),
                secondaries: secondaries
                    .iter()
                    .map(|titulo| crate::ObjectiveDraft {
                        title: titulo.clone(),
                        ..Default::default()
                    })
                    .collect(),
                note: note.clone(),
            };
            let hoje = iniciar_dia(ctx, &input)?;
            let (feitos, total) = hoje.progress();
            let _ = feitos;
            let sessao = hoje
                .session
                .as_ref()
                .map(|sessao| sessao.id.to_string())
                .unwrap_or_default();
            Ok(crate::ActionEffect::new(
                format!(
                    "Dia iniciado com {total} {}. Principal: {main}.",
                    if total == 1 { "objetivo" } else { "objetivos" }
                ),
                // SEM desfazer, e a ausencia e a decisao. O inverso de comecar o
                // dia nao e apagar o dia: seria destruir o unico registro de que
                // ele existiu, e todo Undo que o M/OS oferece e restauracao de
                // estado (ADR-035). Quem comecou por engano encerra — que e uma
                // decisao, e nao um desfazer.
                None,
            )
            .touching("daily_session", sessao, hoje.day.to_string()))
        }
        crate::ActionArgs::DayAddObjective {
            title,
            priority,
            link,
        } => {
            let draft = match link {
                Some(alvo) => {
                    let (kind, id) = resolve_objective_link(state, alvo)?;
                    crate::ObjectiveDraft {
                        title: title.clone(),
                        link_kind: kind,
                        link_id: id,
                        ..Default::default()
                    }
                }
                None => crate::ObjectiveDraft {
                    title: title.clone(),
                    ..Default::default()
                },
            };
            let prioridade = crate::ObjectivePriority::parse(priority)?;
            let hoje = state.daily.add_objective(
                &crate::Day::from_local(ctx.agora_local),
                &draft,
                prioridade,
            )?;
            let criado = hoje
                .objectives
                .iter()
                .find(|objetivo| objetivo.title == draft.title)
                .map(|objetivo| objetivo.id.to_string())
                .unwrap_or_default();
            ctx.capacidades.avisar("data-changed", "daily");
            Ok(crate::ActionEffect::new(
                format!(
                    "\"{title}\" entrou no dia como {}.",
                    if prioridade == crate::ObjectivePriority::Main {
                        "principal"
                    } else {
                        "secundário"
                    }
                ),
                (!criado.is_empty())
                    .then(|| crate::UndoStep::RemoveDailyObjective { id: criado.clone() }),
            )
            .touching("daily_objective", criado, title.clone()))
        }
        crate::ActionArgs::DaySetObjective { objective, status } => {
            // O estado anterior e lido ANTES da mudanca: depois nao ha de onde
            // tirar, e sem ele resolver seria a unica acao do dia sem volta.
            let alvo = resolver_objetivo(ctx, objective)?;
            let anterior = alvo.status;
            let destino = crate::ObjectiveStatus::parse(status)?;
            state.daily.set_objective_status(alvo.id, destino)?;
            ctx.capacidades.avisar("data-changed", "daily");
            Ok(crate::ActionEffect::new(
                format!(
                    "\"{}\" {}.",
                    alvo.title,
                    match destino {
                        crate::ObjectiveStatus::Completed => "concluído",
                        crate::ObjectiveStatus::CarriedOver => "vai para amanhã",
                        crate::ObjectiveStatus::Dropped => "abandonado",
                        crate::ObjectiveStatus::Pending => "voltou a pendente",
                    }
                ),
                Some(crate::UndoStep::RestoreObjectiveStatus {
                    id: alvo.id.to_string(),
                    status: anterior.as_str().to_owned(),
                }),
            )
            .touching("daily_objective", alvo.id.to_string(), alvo.title))
        }
        crate::ActionArgs::DaySetMain { objective } => {
            let alvo = resolver_objetivo(ctx, objective)?;
            let anterior = state
                .daily
                .today(&crate::Day::from_local(ctx.agora_local))?
                .main()
                .map(|principal| principal.id.to_string());
            state.daily.set_main(alvo.id)?;
            ctx.capacidades.avisar("data-changed", "daily");
            Ok(crate::ActionEffect::new(
                format!("\"{}\" agora é o objetivo principal de hoje.", alvo.title),
                Some(crate::UndoStep::RestoreDailyMain {
                    // O anterior so entra quando NAO e o proprio: promover quem
                    // ja era principal e um no-op, e guardar ele como "anterior"
                    // faria o desfazer se promover de volta e parecer que fez
                    // algo.
                    previous_id: anterior.filter(|id| *id != alvo.id.to_string()),
                    demote_id: alvo.id.to_string(),
                }),
            )
            .touching("daily_objective", alvo.id.to_string(), alvo.title))
        }
        crate::ActionArgs::DayEnd { mood, summary } => {
            let hoje = state
                .daily
                .today(&crate::Day::from_local(ctx.agora_local))?;
            let sessao = hoje
                .session
                .as_ref()
                .map(|sessao| sessao.id)
                .ok_or_else(|| {
                    CoreError::new(
                        crate::ErrorCode::InvalidInput,
                        "O dia ainda nao comecou, entao nao ha o que encerrar.",
                        false,
                    )
                })?;
            let (feitos, total) = hoje.progress();
            // Os pendentes ficam PENDENTES: o Hermes nao decide destino de
            // objetivo por ninguem. Eles reaparecem no carry-over de amanha, que
            // e onde a pessoa escolhe.
            let input = crate::EndDayInput {
                resolutions: Vec::new(),
                mood: mood.clone(),
                summary: summary.clone(),
            };
            state.daily.end_session(sessao, &input)?;
            ctx.capacidades.avisar("data-changed", "daily");
            Ok(crate::ActionEffect::new(
                format!("Dia encerrado. {feitos} de {total} objetivos concluídos."),
                Some(crate::UndoStep::ReopenDay {
                    session_id: sessao.to_string(),
                }),
            )
            .touching("daily_session", sessao.to_string(), hoje.day.to_string()))
        }
        // Efeito externo: so o adapter que tem a ponte executa (ver
        // `jarvis::action_resolve`). Chegar aqui e estar num aparelho sem ela.
        crate::ActionArgs::MFinanceCreateBill { .. }
        | crate::ActionArgs::MFinanceCreateCardExpense { .. }
        | crate::ActionArgs::MFinanceCreateIncome { .. }
        | crate::ActionArgs::MFinanceMarkBillPaid { .. }
        | crate::ActionArgs::MFinanceMarkInvoicePaid { .. }
        | crate::ActionArgs::MFinanceCreateSubscription { .. }
        | crate::ActionArgs::MFinanceCreateGoal { .. }
        | crate::ActionArgs::MFinanceUpdateGoal { .. }
        | crate::ActionArgs::MFinanceSetPolicy { .. }
        | crate::ActionArgs::MFinanceUpdateSubscription { .. }
        | crate::ActionArgs::MFinanceCancelSubscription { .. }
        | crate::ActionArgs::MFinanceMarkIncomeReceived { .. }
        | crate::ActionArgs::MFinanceAddGoalContribution { .. }
        | crate::ActionArgs::MFinanceSetGoalStatus { .. }
        | crate::ActionArgs::MFinanceSetInvoiceAmount { .. }
        | crate::ActionArgs::MFinanceSetBudget { .. } => Err(indisponivel(
            "Acao do M-Finance so funciona no aparelho com a ponte do M-Finance.",
        )),
    }
}

/// O mesmo `resolve_project`, ja tipado.
///
/// As acoes de tempo precisam de `ProjectId` e nao de texto, e a ambiguidade
/// ("bate com tres Projects") continua sendo recusada la — um cronometro
/// iniciado no Project errado e hora que vai para a fatura errada.
fn resolve_project_id(state: &Servicos, name: &str) -> Result<crate::ProjectId, CoreError> {
    let id = resolve_project(state, name)?.ok_or_else(|| {
        CoreError::new(
            crate::ErrorCode::NotFound,
            format!("Nao achei um Project chamado \"{name}\"."),
            false,
        )
    })?;
    crate::ProjectId::parse(&id)
}

/// Vazio vira `other`: a atividade e opcional, e recusar por omissao faria o
/// Hermes ter de adivinhar um campo que o usuario nao disse.
fn parse_activity(value: &str) -> Result<crate::ActivityType, CoreError> {
    if value.is_empty() {
        return Ok(crate::ActivityType::Other);
    }
    crate::ActivityType::parse(value)
}

/// O nome do Project, para o recibo falar como o usuario fala.
fn project_name(state: &Servicos, id: crate::ProjectId) -> String {
    state
        .work
        .projects(true)
        .ok()
        .and_then(|projects| projects.into_iter().find(|project| project.id == id))
        .map(|project| project.name)
        .unwrap_or_else(|| "Project".to_owned())
}

/// `2h30` ou `45min`, para o recibo.
fn spoken_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h{minutes:02}")
    } else {
        format!("{minutes}min")
    }
}

/// O instante de um dia `AAAA-MM-DD`, ao meio-dia local.
///
/// Meio-dia, e nao meia-noite: uma hora lancada a meia-noite fica na fronteira
/// entre dois dias, e qualquer diferenca de fuso a joga para o dia anterior. Ao
/// meio-dia sobram doze horas de folga para cada lado.
fn moment_for_day(day: &str) -> Result<time::OffsetDateTime, CoreError> {
    if day.is_empty() {
        return Ok(time::OffsetDateTime::now_utc());
    }
    crate::parse_moment(&format!("{day}T12:00:00Z"))
}

/// Desfaz uma acao executada.
///
/// Devolve uma nota quando o desfazer foi PARCIAL — o que ficou como estava e
/// por que. Vazia quando tudo voltou.
pub fn desfazer_acao(ctx: &Contexto<'_>, step: crate::UndoStep) -> Result<String, CoreError> {
    let state = ctx.servicos;
    let mut nota = String::new();
    match step {
        crate::UndoStep::UndoTaskStart(passo) => {
            let desfeito = state.work.undo_task_start(&passo)?;
            nota = desfeito.notes.join(" ");
        }
        crate::UndoStep::ArchiveCapture { id } => {
            state.captures.archive(&id)?;
        }
        crate::UndoStep::ArchiveTask { id } => {
            state.work.set_task_archived(&id, true)?;
        }
        crate::UndoStep::ArchiveProject { id } => {
            state.work.set_project_archived(&id, true)?;
        }
        crate::UndoStep::ArchiveResource { id } => {
            state
                .memory
                .set_resource_lifecycle(&id, crate::LifecycleState::Archived)?;
        }
        crate::UndoStep::RestoreTaskState { id, state: estado } => {
            state
                .work
                .set_task_state(&id, crate::TaskState::parse(&estado)?)?;
        }
        crate::UndoStep::RestoreTaskProject { id, project_id } => {
            let task = state.work.task(&id)?;
            state.work.update_task(crate::UpdateTaskInput {
                project_id,
                ..crate::UpdateTaskInput::from_task(&task)
            })?;
        }
        crate::UndoStep::RemoveChecklistItems { ids } => {
            for id in ids {
                state.work.delete_checklist_item(&id)?;
            }
        }
        crate::UndoStep::RestoreChecklistItem { id, done } => {
            state.work.set_checklist_item_done(&id, done)?;
        }
        crate::UndoStep::RestoreTaskPlan {
            id,
            due_at,
            priority,
            estimate_minutes,
            waiting_for,
            follow_up_at,
        } => {
            let task = state.work.task(&id)?;
            state.work.update_task(crate::UpdateTaskInput {
                due_at,
                priority: Some(priority),
                estimate_minutes,
                waiting_for,
                follow_up_at,
                ..crate::UpdateTaskInput::from_task(&task)
            })?;
        }
        crate::UndoStep::CancelReminder { id } => {
            state
                .attention
                .transition(crate::ReminderId::parse(&id)?, crate::Transition::Cancel)?;
            ctx.capacidades.lembretes_mudaram();
        }
        crate::UndoStep::UndoCaptureToTask {
            capture_id,
            task_id,
        } => {
            state.work.set_task_archived(&task_id, true)?;
            // A Capture volta para a Inbox, e nao para o arquivo: o que se
            // desfez foi a DECISAO sobre ela, e o lugar de uma Capture ainda
            // nao decidida e a Inbox.
            state.captures.move_to_inbox(&capture_id)?;
        }
        crate::UndoStep::TrashTimeEntry { id } => {
            state.tracking.trash(crate::TimeEntryId::parse(&id)?)?;
        }
        crate::UndoStep::UndoMeetingInsight {
            insight_id,
            task_id,
            reminder_id,
            task_created,
        } => {
            // A ordem e deliberada: primeiro tirar da vista o que TOCA. Um
            // lembrete que dispara enquanto o desfazer roda avisaria sobre uma
            // Task que a pessoa acabou de dizer que nao queria.
            if let Some(reminder_id) = reminder_id {
                state.attention.transition(
                    crate::ReminderId::parse(&reminder_id)?,
                    crate::Transition::Cancel,
                )?;
            }
            // So arquiva a Task que o ACEITE criou. Uma Task que ja existia e
            // foi apenas ligada ao item continua da pessoa (ADR-086).
            if task_created {
                state.work.set_task_archived(&task_id, true)?;
            }
            // Por ultimo o item volta a ser oferecido — e isso e o ponto: quem
            // desfez provavelmente quer refazer diferente.
            state.meetings.reopen_insight(&insight_id)?;
        }
        crate::UndoStep::RemoveDailyObjective { id } => {
            state
                .daily
                .remove_objective(crate::DailyObjectiveId::parse(&id)?)?;
        }
        crate::UndoStep::RestoreObjectiveStatus { id, status } => {
            state.daily.set_objective_status(
                crate::DailyObjectiveId::parse(&id)?,
                crate::ObjectiveStatus::parse(&status)?,
            )?;
        }
        crate::UndoStep::RestoreDailyMain {
            previous_id,
            demote_id,
        } => match previous_id {
            // Promover o anterior REBAIXA o novo na mesma transacao — e o que
            // `set_main_objective` garante —, entao um passo basta.
            Some(previous_id) => {
                state
                    .daily
                    .set_main(crate::DailyObjectiveId::parse(&previous_id)?)?;
            }
            // Nao havia principal antes: desfazer e REBAIXAR, e nao promover
            // ninguem. Sem este braco, um dia que nao tinha principal ganharia
            // um pelo desfazer.
            None => {
                let alvo = crate::DailyObjectiveId::parse(&demote_id)?;
                state.daily.set_secondary(alvo)?;
            }
        },
        crate::UndoStep::ReopenDay { session_id } => {
            state
                .daily
                .reopen(crate::DailySessionId::parse(&session_id)?)?;
        }
        crate::UndoStep::UndoVoiceAction {
            capture_id,
            task_id,
            reminder_id,
        } => {
            // Mesma ordem do desfazer de reuniao: primeiro sai de vista o que
            // TOCA. Um lembrete que dispara no meio do desfazer avisaria sobre
            // uma Task que a pessoa acabou de dizer que nao queria.
            if let Some(reminder_id) = reminder_id {
                state.attention.transition(
                    crate::ReminderId::parse(&reminder_id)?,
                    crate::Transition::Cancel,
                )?;
            }
            state.work.set_task_archived(&task_id, true)?;
            // A fala volta para a Inbox. Ela nao e apagada: desfazer a acao nao
            // desfaz o ter falado, e o lugar de uma fala ainda nao decidida e a
            // Inbox.
            state.captures.move_to_inbox(&capture_id)?;
        }
    }
    ctx.capacidades.avisar("data-changed", "undo");
    Ok(nota)
}

/// Acha uma entidade a partir do que o modelo escreveu.
///
/// # O que mudou aqui, e por que
///
/// Antes cada acao tinha a propria comparacao, e todas faziam a mesma coisa:
/// `to_lowercase().contains()` sobre o nome. Isso resolvia "Minarum" e nao
/// resolvia mais nada — nem o id que o proprio M/OS acabara de mandar no bloco
/// de candidatos, nem um titulo escrito com acento diferente.
///
/// Agora a comparacao e uma so, em `crate::resolve`, e ela le em degraus:
/// id inteiro, prefixo de id, titulo exato, comeco de titulo, pedaco de titulo.
/// O primeiro degrau que acerta decide. E a diferenca entre o Hermes citar
/// `7c3e2b19` — o id que ele leu nos candidatos — e o M/OS responder "nao achei".
///
/// Ambiguidade continua RECUSANDO em vez de escolher o primeiro: "Escadas"
/// batendo em dois Projects e exatamente o caso onde adivinhar cria a Task no
/// lugar errado, e o erro so aparece dias depois.
fn resolve_project(state: &Servicos, name: &str) -> Result<Option<String>, CoreError> {
    let projects = state.work.projects(false)?;
    let found = crate::resolve(
        &projects,
        name,
        |project| project.id.to_string(),
        |project| project.name.clone(),
    );
    if let Some(error) = crate::resolution_error(
        &found,
        crate::EntityKind::Project,
        name,
        |project: &crate::Project| project.name.clone(),
    ) {
        return Err(error);
    }
    Ok(found.one().map(|project| project.id.to_string()))
}

/// Devolve a Task inteira, e nao so o id: quem chama precisa do estado ANTERIOR
/// para montar o desfazer, e ele ja esta aqui. Buscar de novo depois custaria
/// uma segunda varredura e leria um estado que a mudanca ja alterou.
fn resolve_task(state: &Servicos, title: &str) -> Result<crate::Task, CoreError> {
    let tasks = state.work.tasks(false)?;
    let found = crate::resolve(
        &tasks,
        title,
        |task| task.id.to_string(),
        |task| task.title.clone(),
    );
    match crate::resolution_error(
        &found,
        crate::EntityKind::Task,
        title,
        |task: &crate::Task| task.title.clone(),
    ) {
        Some(error) => Err(error),
        None => Ok(found.one().expect("sem erro ha exatamente um").clone()),
    }
}

/// A Capture, pelo id ou por um pedaco do que esta escrito nela.
fn resolve_capture(state: &Servicos, reference: &str) -> Result<crate::Capture, CoreError> {
    // A Inbox primeiro, e a base inteira depois. Nao e otimizacao: "essa
    // captura" quase sempre quer dizer uma das que ainda esperam decisao, e
    // procurar na Inbox primeiro faz o pedaco de texto que tambem bate com
    // Captures antigas resolver na que importa.
    let inbox = state.captures.inbox(200)?;
    let found = crate::resolve(
        &inbox,
        reference,
        |capture| capture.id.to_string(),
        |capture| capture.content.clone(),
    );
    if let crate::Resolved::One(capture) = found {
        return Ok(capture.clone());
    }

    let todas = state.captures.recent(200)?;
    let found = crate::resolve(
        &todas,
        reference,
        |capture| capture.id.to_string(),
        |capture| capture.content.clone(),
    );
    match crate::resolution_error(
        &found,
        crate::EntityKind::Capture,
        reference,
        |capture: &crate::Capture| capture.content.clone(),
    ) {
        Some(error) => Err(error),
        None => Ok(found.one().expect("sem erro ha exatamente um").clone()),
    }
}

/// O lembrete, pelo id ou pelo titulo.
///
/// So os abertos. Um lembrete ja concluido nao e o que alguem quer dizer com
/// "cancela aquele lembrete", e incluir os fechados faria o titulo repetido de
/// um lembrete recorrente virar ambiguidade toda vez.
fn resolve_reminder(state: &Servicos, reference: &str) -> Result<crate::Reminder, CoreError> {
    let abertos = state.attention.open()?;
    let found = crate::resolve(
        &abertos,
        reference,
        |reminder| reminder.id.to_string(),
        |reminder| reminder.title.clone(),
    );
    match crate::resolution_error(
        &found,
        crate::EntityKind::Reminder,
        reference,
        |reminder: &crate::Reminder| reminder.title.clone(),
    ) {
        Some(error) => Err(error),
        None => Ok(found.one().expect("sem erro ha exatamente um").clone()),
    }
}

/// O vinculo de um objetivo, resolvido para as duas colunas que o banco guarda.
///
/// Reaproveita os mesmos resolvedores do `resolve_target`, mas com o conjunto
/// mais estreito de `LinkKind`: um objetivo do dia ligado a uma Conversa ou a um
/// App nao quer dizer nada, e recusar aqui e mais barato que descobrir na tela.
fn resolve_objective_link(
    state: &Servicos,
    target: &crate::TargetRef,
) -> Result<(String, String), CoreError> {
    match target.kind.as_str() {
        "task" => Ok((
            "task".to_owned(),
            resolve_task(state, &target.reference)?.id.to_string(),
        )),
        "project" => {
            let id = resolve_project(state, &target.reference)?.ok_or_else(|| {
                CoreError::new(
                    crate::ErrorCode::NotFound,
                    format!("Nao achei Project para \"{}\".", target.reference),
                    false,
                )
            })?;
            Ok(("project".to_owned(), id))
        }
        "capture" => Ok((
            "capture".to_owned(),
            resolve_capture(state, &target.reference)?.id.to_string(),
        )),
        outro => Err(CoreError::new(
            crate::ErrorCode::InvalidInput,
            format!("`{outro}` nao e um tipo de vinculo de objetivo do dia."),
            false,
        )),
    }
}

/// O alvo de um lembrete, resolvido para o enum do dominio.
///
/// Devolve tambem o rotulo, porque o recibo fala como o usuario fala:
/// "vinculado a Task Enviar bases" e util, "vinculado a 7c3e2b19" nao e.
fn resolve_target(
    state: &Servicos,
    target: &crate::TargetRef,
) -> Result<(crate::ReminderTarget, String), CoreError> {
    match target.kind.as_str() {
        "task" => {
            let task = resolve_task(state, &target.reference)?;
            Ok((crate::ReminderTarget::Task(task.id), task.title))
        }
        "project" => {
            let id = resolve_project(state, &target.reference)?.ok_or_else(|| {
                CoreError::new(
                    crate::ErrorCode::NotFound,
                    format!("Nao achei Project para \"{}\".", target.reference),
                    false,
                )
            })?;
            let id = crate::ProjectId::parse(&id)?;
            Ok((crate::ReminderTarget::Project(id), project_name(state, id)))
        }
        "capture" => {
            let capture = resolve_capture(state, &target.reference)?;
            Ok((crate::ReminderTarget::Capture(capture.id), capture.content))
        }
        "resource" => {
            let resources = state.memory.resources(false)?;
            let found = crate::resolve(
                &resources,
                &target.reference,
                |resource| resource.id.to_string(),
                |resource| resource.title.clone(),
            );
            match crate::resolution_error(
                &found,
                crate::EntityKind::Resource,
                &target.reference,
                |resource: &crate::Resource| resource.title.clone(),
            ) {
                Some(error) => Err(error),
                None => {
                    let resource = found.one().expect("sem erro ha exatamente um");
                    Ok((
                        crate::ReminderTarget::Resource(resource.id),
                        resource.title.clone(),
                    ))
                }
            }
        }
        outro => Err(CoreError::new(
            crate::ErrorCode::InvalidInput,
            format!("`{outro}` nao e um tipo de alvo de lembrete."),
            false,
        )),
    }
}

// ------------------------------------------------------------------ o dia

/// O rotulo de uma entidade vinculada, para completar um objetivo sem titulo.
///
/// Um objetivo pode nascer sem titulo quando ele E uma Task ou um Project: a
/// pessoa escolheu a Task no seletor, e digitar o titulo de novo seria trabalho
/// que o sistema ja tem como fazer. Falha vira `None`, e ai o dominio recusa o
/// objetivo sem titulo — melhor que gravar "(sem titulo)".
pub fn rotulo_do_vinculo(state: &Servicos, link: &crate::ObjectiveLink) -> Option<String> {
    match link.kind {
        crate::LinkKind::Task => state.work.task(&link.id).ok().map(|task| task.title),
        crate::LinkKind::Project => state
            .work
            .project(&link.id)
            .ok()
            .map(|project| project.name),
        crate::LinkKind::Capture => state.captures.get(&link.id).ok().map(|c| c.content),
        crate::LinkKind::Resource => state.memory.resource(&link.id).ok().map(|r| r.title),
        crate::LinkKind::Meeting => state.meetings.meeting(&link.id).ok().map(|m| m.title),
    }
}

/// Completa o titulo de um rascunho a partir do vinculo, quando ele veio vazio.
pub fn completar_rascunho(
    state: &Servicos,
    draft: &crate::ObjectiveDraft,
) -> Result<crate::ObjectiveDraft, CoreError> {
    if !draft.title.trim().is_empty() {
        return Ok(draft.clone());
    }
    let Some(link) = draft.link()? else {
        return Err(indisponivel("O objetivo precisa de um titulo."));
    };
    let Some(titulo) = rotulo_do_vinculo(state, &link) else {
        return Err(indisponivel(
            "Nao achei o que este objetivo aponta. Escreva um titulo.",
        ));
    };
    Ok(crate::ObjectiveDraft {
        title: titulo,
        ..draft.clone()
    })
}

/// Comeca o dia de `agora_local`. Chamado pela interface E pelo agente.
pub fn iniciar_dia(
    ctx: &Contexto<'_>,
    input: &crate::StartDayInput,
) -> Result<crate::DailyToday, CoreError> {
    let state = ctx.servicos;
    let resolvido = crate::StartDayInput {
        main: input
            .main
            .as_ref()
            .map(|draft| completar_rascunho(state, draft))
            .transpose()?,
        secondaries: input
            .secondaries
            .iter()
            .map(|draft| completar_rascunho(state, draft))
            .collect::<Result<Vec<_>, _>>()?,
        note: input.note.clone(),
    };
    let hoje = state
        .daily
        .start(crate::Day::from_local(ctx.agora_local), &resolvido)?;
    ctx.capacidades.avisar("data-changed", "daily");
    Ok(hoje)
}

/// Acha um objetivo DE HOJE pela referencia que o agente escreveu.
///
/// So de hoje: resolver um objetivo de semanas atras por semelhanca de titulo
/// seria agir sobre o dia errado — sem que nada na tela mostrasse isso.
pub fn resolver_objetivo(
    ctx: &Contexto<'_>,
    referencia: &str,
) -> Result<crate::DailyObjective, CoreError> {
    let hoje = ctx
        .servicos
        .daily
        .today(&crate::Day::from_local(ctx.agora_local))?;
    if hoje.objectives.is_empty() {
        return Err(indisponivel(
            "O dia ainda nao comecou, entao nao ha objetivo para mudar.",
        ));
    }
    let achado = crate::resolve(
        &hoje.objectives,
        referencia,
        |objetivo| objetivo.id.to_string(),
        |objetivo| objetivo.title.clone(),
    );
    match crate::resolution_error(
        &achado,
        crate::EntityKind::DailyObjective,
        referencia,
        |objetivo: &crate::DailyObjective| objetivo.title.clone(),
    ) {
        Some(erro) => Err(erro),
        None => Ok(achado.one().expect("sem erro ha exatamente um").clone()),
    }
}
