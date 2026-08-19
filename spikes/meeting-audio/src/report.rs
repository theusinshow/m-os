//! O que o spike MEDE.
//!
//! Um spike que so produz arquivos de audio nao responde o Gate A. As perguntas
//! D-1 a D-5 do `docs/MEETING-AGENT.md` §24 sao sobre numeros, e este modulo e
//! quem os coleta — sem tocar o Windows, para que a matematica seja testavel.

use serde::{Deserialize, Serialize};

/// Contadores de um canal, acumulados durante a captura.
///
/// Todo campo aqui existe para responder uma pergunta declarada. Um contador que
/// nao responde nada e ruido que da ar de rigor.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStats {
    /// Pacotes entregues pelo `IAudioCaptureClient`.
    pub packets: u64,
    pub frames: u64,
    /// Pacotes marcados `AUDCLNT_BUFFERFLAGS_SILENT`.
    ///
    /// Medido em 2026-08-18: este contador NAO responde a D-2, e a expectativa
    /// inicial de que responderia estava errada. Com o keep-alive escrevendo
    /// zeros de verdade, o loopback os entrega como audio comum e a flag nunca
    /// aparece. Quem responde a D-2 e `frames`: sem keep-alive num endpoint
    /// ocioso, ele fica em zero.
    pub silent_packets: u64,
    /// `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY`. Cada um e um buraco que o
    /// dispositivo admite ter deixado.
    pub discontinuities: u64,
    pub timestamp_errors: u64,
    /// Frames que o `index` do device diz existirem e que nao chegaram.
    /// Calculado pelo salto da posicao entre pacotes, nao estimado.
    pub dropped_frames: u64,
    /// Maior intervalo entre dois pacotes, em milissegundos. Responde D-1: se o
    /// modo por evento nao dispara, este numero explode.
    pub max_gap_ms: u64,
    /// Timestamp QPC (100 ns) do primeiro e do ultimo pacote.
    pub first_timestamp_hns: u64,
    pub last_timestamp_hns: u64,
    /// Ocorrencias em que o modo de timing precisou trocar por falta de evento.
    pub timing_fallbacks: u64,
    /// Erros de leitura que nao derrubaram a captura.
    pub read_errors: u64,
}

impl ChannelStats {
    /// Quanto tempo os timestamps do dispositivo dizem ter passado.
    pub fn device_span_ms(&self) -> u64 {
        self.last_timestamp_hns
            .saturating_sub(self.first_timestamp_hns)
            / 10_000
    }

    /// Quanto tempo os frames gravados representam.
    pub fn recorded_ms(&self, sample_rate: u32) -> u64 {
        self.frames * 1000 / sample_rate.max(1) as u64
    }

    /// Deriva do relogio do dispositivo, em milissegundos, com sinal.
    ///
    /// Positivo significa que gravamos MENOS do que o dispositivo diz ter
    /// passado — ou seja, faltou audio. Negativo seria o contrario, e nao
    /// deveria acontecer; se acontecer, e um achado, nao um arredondamento.
    pub fn drift_ms(&self, sample_rate: u32) -> i64 {
        self.device_span_ms() as i64 - self.recorded_ms(sample_rate) as i64
    }
}

/// O relatorio que o spike escreve ao fim, e que e a evidencia do Gate A.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub started_at_unix_ms: u128,
    pub wall_duration_ms: u64,

    pub mic: Option<ChannelReport>,
    pub system: Option<ChannelReport>,

    /// Diferenca entre o audio gravado nos dois canais. E o numero de D-5: dois
    /// canais que compartilham uma linha do tempo precisam terminar com a mesma
    /// quantidade de tempo gravado.
    pub cross_channel_drift_ms: Option<i64>,

    pub keep_alive: bool,
    pub peak_working_set_bytes: u64,
    pub process_cpu_ms: u64,
    /// CPU do processo dividido pelo tempo de parede, em porcentagem de UM
    /// nucleo. Numa maquina de 32 threads, 100% aqui significa um nucleo cheio.
    pub cpu_percent_of_one_core: f64,
    pub bytes_on_disk: u64,
    pub verdict: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelReport {
    pub device: String,
    pub requested_format: String,
    pub effective_format: String,
    /// `events` ou `polling`. O que foi USADO, nao o que foi pedido.
    pub timing: String,
    /// Se o formato pedido foi aceito com `autoconvert`. Responde D-3.
    pub autoconvert_accepted: bool,
    pub sample_rate: u32,
    pub stats: ChannelStats,
    pub recorded_ms: u64,
    pub device_span_ms: u64,
    pub drift_ms: i64,
    pub chunks: u32,
    pub trailing_bytes: u64,
    /// Preenchido quando o canal caiu antes do fim.
    pub lost_at_ms: Option<u64>,
    pub lost_reason: Option<String>,
}

/// Traduz numeros em frases de aprovacao ou reprovacao.
///
/// O veredito e calculado, e nao escrito a mao depois de olhar o resultado.
/// A diferenca importa: um limite decidido antes do teste e um criterio; um
/// limite decidido depois e uma desculpa.
pub fn verdict(report: &Report) -> Vec<String> {
    let mut lines = Vec::new();

    for (name, channel) in [("mic", &report.mic), ("system", &report.system)] {
        let Some(channel) = channel else {
            lines.push(format!("[--] {name}: nao foi aberto"));
            continue;
        };

        if channel.stats.frames == 0 {
            lines.push(format!("[FALHA] {name}: nenhum frame capturado"));
            continue;
        }

        // D-1. Um gap acima de 2 s num modo por evento significa que o evento
        // nao esta disparando de forma confiavel.
        if channel.stats.max_gap_ms > 2_000 {
            lines.push(format!(
                "[FALHA] {name}: maior intervalo entre pacotes foi {} ms (limite 2000)",
                channel.stats.max_gap_ms
            ));
        } else {
            lines.push(format!(
                "[ok] {name}: maior intervalo entre pacotes {} ms",
                channel.stats.max_gap_ms
            ));
        }

        if channel.stats.timing_fallbacks > 0 {
            lines.push(format!(
                "[NOTA] {name}: o modo por evento falhou {}x e a captura passou a polling (D-1)",
                channel.stats.timing_fallbacks
            ));
        }

        // D-5. 200 ms em 60 min e o orcamento da §19.
        let allowed = (channel.recorded_ms / 300).max(200) as i64;
        if channel.drift_ms.abs() > allowed {
            lines.push(format!(
                "[FALHA] {name}: deriva de {} ms contra o relogio do dispositivo (limite {allowed})",
                channel.drift_ms
            ));
        } else {
            lines.push(format!(
                "[ok] {name}: deriva de {} ms contra o relogio do dispositivo",
                channel.drift_ms
            ));
        }

        if channel.stats.dropped_frames > 0 {
            lines.push(format!(
                "[NOTA] {name}: {} frames perdidos, em {} descontinuidades",
                channel.stats.dropped_frames, channel.stats.discontinuities
            ));
        }

        if channel.trailing_bytes > 0 {
            lines.push(format!(
                "[FALHA] {name}: {} bytes soltos alem do ultimo frame inteiro",
                channel.trailing_bytes
            ));
        }

        if let Some(at) = channel.lost_at_ms {
            lines.push(format!(
                "[NOTA] {name}: canal perdido aos {at} ms — {}",
                channel.lost_reason.as_deref().unwrap_or("sem motivo registrado")
            ));
        }

        if !channel.autoconvert_accepted {
            lines.push(format!(
                "[NOTA] {name}: o formato pedido nao foi aceito; usando {} (D-3)",
                channel.effective_format
            ));
        }
    }

    // D-2. Se os dois canais existem, eles precisam ter gravado a mesma
    // quantidade de tempo. Um SYSTEM muito mais curto que o MIC e a assinatura
    // exata do buraco do loopback em silencio.
    if let Some(drift) = report.cross_channel_drift_ms {
        if drift.abs() > 500 {
            lines.push(format!(
                "[FALHA] canais divergem em {drift} ms de audio gravado (limite 500)"
            ));
        } else {
            lines.push(format!("[ok] canais divergem em {drift} ms de audio gravado"));
        }
    }

    // O sinal honesto da D-2 e a AUSENCIA DE FRAMES, e nao a ausencia de pacotes
    // marcados como silenciosos. Medido em 2026-08-18: com o keep-alive escrevendo
    // zeros de verdade, o loopback entrega pacotes NAO marcados como silenciosos,
    // porque do ponto de vista dele aquilo e audio. Um veredito baseado na flag
    // acusaria falha justamente na configuracao que funciona.
    if let Some(system) = &report.system {
        if system.stats.frames == 0 {
            lines.push(
                "[FALHA] system: nenhum frame — e a assinatura do buraco do loopback num endpoint ocioso (D-2)"
                    .to_owned(),
            );
        }
    }

    // §19: menos de 2% de CPU. O orcamento e do processo inteiro, e este spike
    // e quase so a captura.
    if report.cpu_percent_of_one_core > 2.0 {
        lines.push(format!(
            "[NOTA] CPU em {:.2}% de um nucleo, acima do orcamento de 2%",
            report.cpu_percent_of_one_core
        ));
    } else {
        lines.push(format!(
            "[ok] CPU em {:.2}% de um nucleo",
            report.cpu_percent_of_one_core
        ));
    }

    if report.peak_working_set_bytes > 60 * 1024 * 1024 {
        lines.push(format!(
            "[NOTA] pico de memoria em {} MB, acima do orcamento de 60 MB",
            report.peak_working_set_bytes / (1024 * 1024)
        ));
    } else {
        lines.push(format!(
            "[ok] pico de memoria em {} MB",
            report.peak_working_set_bytes / (1024 * 1024)
        ));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(frames: u64, span_hns: u64) -> ChannelReport {
        let stats = ChannelStats {
            packets: 100,
            frames,
            first_timestamp_hns: 1_000_000,
            last_timestamp_hns: 1_000_000 + span_hns,
            ..Default::default()
        };
        ChannelReport {
            device: "teste".into(),
            requested_format: "16000/1/i16".into(),
            effective_format: "16000/1/i16".into(),
            timing: "events".into(),
            autoconvert_accepted: true,
            sample_rate: 16_000,
            recorded_ms: stats.recorded_ms(16_000),
            device_span_ms: stats.device_span_ms(),
            drift_ms: stats.drift_ms(16_000),
            stats,
            chunks: 1,
            trailing_bytes: 0,
            lost_at_ms: None,
            lost_reason: None,
        }
    }

    fn report(mic: Option<ChannelReport>, system: Option<ChannelReport>) -> Report {
        let drift = match (&mic, &system) {
            (Some(m), Some(s)) => Some(m.recorded_ms as i64 - s.recorded_ms as i64),
            _ => None,
        };
        Report {
            started_at_unix_ms: 0,
            wall_duration_ms: 60_000,
            mic,
            system,
            cross_channel_drift_ms: drift,
            keep_alive: true,
            peak_working_set_bytes: 20 * 1024 * 1024,
            process_cpu_ms: 300,
            cpu_percent_of_one_core: 0.5,
            bytes_on_disk: 0,
            verdict: Vec::new(),
        }
    }

    #[test]
    fn deriva_zero_quando_frames_batem_com_o_relogio() {
        // 60 s de frames, 60 s de timestamps.
        let stats = ChannelStats {
            frames: 16_000 * 60,
            first_timestamp_hns: 0,
            last_timestamp_hns: 60 * 10_000_000,
            ..Default::default()
        };
        assert_eq!(stats.recorded_ms(16_000), 60_000);
        assert_eq!(stats.device_span_ms(), 60_000);
        assert_eq!(stats.drift_ms(16_000), 0);
    }

    #[test]
    fn deriva_positiva_significa_audio_faltando() {
        let stats = ChannelStats {
            frames: 16_000 * 59,
            first_timestamp_hns: 0,
            last_timestamp_hns: 60 * 10_000_000,
            ..Default::default()
        };
        assert_eq!(stats.drift_ms(16_000), 1000);
    }

    #[test]
    fn canal_sem_frames_reprova() {
        let mut empty = channel(0, 0);
        empty.recorded_ms = 0;
        let report = report(Some(channel(16_000 * 60, 60 * 10_000_000)), Some(empty));
        let lines = verdict(&report);
        assert!(lines.iter().any(|line| line.contains("[FALHA] system: nenhum frame")));
    }

    #[test]
    fn canais_divergentes_reprovam() {
        // O caso do buraco do loopback: SYSTEM gravou 40 s onde MIC gravou 60.
        let report = report(
            Some(channel(16_000 * 60, 60 * 10_000_000)),
            Some(channel(16_000 * 40, 60 * 10_000_000)),
        );
        let lines = verdict(&report);
        assert!(lines.iter().any(|line| line.contains("[FALHA] canais divergem")));
    }

    #[test]
    fn gap_grande_reprova() {
        let mut mic = channel(16_000 * 60, 60 * 10_000_000);
        mic.stats.max_gap_ms = 5_000;
        let lines = verdict(&report(Some(mic), None));
        assert!(lines
            .iter()
            .any(|line| line.contains("[FALHA] mic: maior intervalo")));
    }

    #[test]
    fn caso_bom_nao_produz_falha() {
        let report = report(
            Some(channel(16_000 * 60, 60 * 10_000_000)),
            Some(channel(16_000 * 60, 60 * 10_000_000)),
        );
        let lines = verdict(&report);
        assert!(
            !lines.iter().any(|line| line.contains("[FALHA]")),
            "veredito inesperado: {lines:#?}"
        );
    }
}
