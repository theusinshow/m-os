//! O relogio que ordena eventos entre dispositivos.
//!
//! # Por que nao `OffsetDateTime`
//!
//! Dois dispositivos nao concordam sobre que horas sao. O relogio do celular
//! anda, o do PC atrasa, o fuso muda, alguem acerta a hora a mao. Ordenar
//! eventos por timestamp de parede significa que **um relogio errado apaga o
//! trabalho de quem estava certo** — e essa e exatamente a perda silenciosa que
//! o §8 da missao proibe.
//!
//! # O que e um HLC
//!
//! Hybrid Logical Clock: hora de parede + contador + identidade do dispositivo.
//!
//! - A hora de parede mantem o valor **legivel para humanos** e proximo do
//!   tempo real, que e o que faz a Timeline (§25) fazer sentido.
//! - O contador garante ordem total quando dois eventos caem no mesmo
//!   milissegundo, ou quando um relogio esta atrasado: ao observar um evento do
//!   futuro, o relogio local **sobe junto** em vez de reordenar o passado.
//! - A identidade do dispositivo desempata o resto, de forma deterministica:
//!   os dois lados chegam a mesma ordem sem precisar conversar.
//!
//! O resultado e uma ordem total, estavel e sem servidor arbitro. E o que
//! permite ao M/OS continuar `offline-first` (§6) sem depender de ninguem para
//! decidir quem veio primeiro.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::DeviceId;

/// Um instante ordenavel entre dispositivos.
///
/// A ordem e: hora de parede, depois contador, depois dispositivo. Os tres
/// campos participam, e por isso `Ord` e total — nunca ha empate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hlc {
    /// Milissegundos desde a epoca. Milissegundo e nao nanossegundo porque o
    /// contador ja resolve a colisao, e um numero menor viaja melhor em JSON.
    pub wall_ms: i64,
    /// Quantos eventos ja aconteceram neste mesmo `wall_ms`.
    pub counter: u32,
    /// Quem gerou. Desempate final, e tambem a resposta para "de onde veio?".
    pub device: DeviceId,
}

impl Hlc {
    pub fn new(wall_ms: i64, counter: u32, device: DeviceId) -> Self {
        Self {
            wall_ms,
            counter,
            device,
        }
    }
}

impl PartialOrd for Hlc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hlc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wall_ms
            .cmp(&other.wall_ms)
            .then(self.counter.cmp(&other.counter))
            // O desempate por dispositivo e arbitrario DE PROPOSITO: o que
            // importa nao e qual dispositivo ganha, e sim que os dois cheguem a
            // MESMA conclusao sem se falarem.
            .then(self.device.cmp(&other.device))
    }
}

/// O relogio de um dispositivo.
///
/// Guarda o ultimo instante emitido para nunca voltar atras, mesmo que o
/// relogio do sistema volte — o que acontece de verdade em horario de verao,
/// em sincronizacao NTP e em maquina virtual que hiberna.
#[derive(Clone, Debug)]
pub struct HlcClock {
    device: DeviceId,
    ultimo: Hlc,
}

impl HlcClock {
    pub fn new(device: DeviceId) -> Self {
        Self {
            device,
            ultimo: Hlc::new(0, 0, device),
        }
    }

    /// Um instante novo, sempre maior que todos os anteriores deste relogio.
    ///
    /// `agora_ms` vem de fora porque o dominio nao le relogio — quem le e o
    /// adaptador. E o mesmo motivo pelo qual `mos-core` tem `clock.rs`: teste
    /// que depende do relogio de parede e teste que falha na virada do ano.
    pub fn tick(&mut self, agora_ms: i64) -> Hlc {
        let proximo = if agora_ms > self.ultimo.wall_ms {
            Hlc::new(agora_ms, 0, self.device)
        } else {
            // O relogio de parede nao avancou (ou voltou). O contador segura a
            // ordem ate ele alcancar.
            Hlc::new(self.ultimo.wall_ms, self.ultimo.counter + 1, self.device)
        };
        self.ultimo = proximo;
        proximo
    }

    /// Absorve um instante que chegou de outro dispositivo.
    ///
    /// Depois disto, todo `tick` deste relogio vem DEPOIS do que foi observado.
    /// E o que impede um celular atrasado de gerar eventos que se ordenam antes
    /// de coisas que ele acabou de receber e ja mostrou na tela.
    pub fn observar(&mut self, remoto: Hlc, agora_ms: i64) -> Hlc {
        let base = if remoto.wall_ms > self.ultimo.wall_ms {
            Hlc::new(remoto.wall_ms, remoto.counter, self.device)
        } else {
            self.ultimo
        };
        self.ultimo = base;
        // `max` e o ponto: o tick sai depois do relogio de parede E depois do
        // que veio de fora, e e isso que impede um aparelho atrasado de emitir
        // algo que se ordena antes do que ele acabou de receber.
        self.tick(agora_ms.max(base.wall_ms))
    }

    pub fn device(&self) -> DeviceId {
        self.device
    }

    /// O ultimo instante emitido. E o que se persiste entre execucoes: sem
    /// isso, reabrir o app com o relogio atrasado geraria eventos no passado.
    pub fn ultimo(&self) -> Hlc {
        self.ultimo
    }

    /// Restaura um relogio a partir do que foi persistido.
    pub fn restaurar(device: DeviceId, ultimo: Hlc) -> Self {
        Self { device, ultimo }
    }
}
