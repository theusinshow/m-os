//! De onde o domínio lê o tempo.
//!
//! Existe para que regra temporal seja testável sem esperar. Sem isto, testar
//! "o lembrete perdido durante o sono é recuperado" exigiria dormir de verdade,
//! e testar horário de verão exigiria mudar o relógio da máquina — coisas que
//! ninguém faz, e por isso regras temporais costumam ir para produção sem teste.
//!
//! **Duas leituras, e não uma.** `now` é o relógio de parede: sofre ajuste
//! manual, NTP e horário de verão. `monotonic` não anda para trás e não salta.
//! É a DIFERENÇA entre as duas que revela que a máquina dormiu ou que o relógio
//! mudou — e é assim que o agendador descobre isso sem depender de um evento de
//! sistema operacional que a stack atual não expõe (`ATTENTION-SYSTEM.md` §28).
//!
//! Os `OffsetDateTime::now_utc()` que já existem no crate NÃO passam por aqui.
//! Eles carimbam `created_at` e afins, onde o instante é registro e não decisão.
//! Esta porta é para quem decide.

use std::time::Instant;

use time::OffsetDateTime;

pub trait Clock: Send + Sync {
    /// Instante atual em UTC. Relógio de parede.
    fn now(&self) -> OffsetDateTime;

    /// Leitura monotônica, para medir decurso sem sofrer salto de relógio.
    fn monotonic(&self) -> Instant;
}

/// O relógio de verdade.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn monotonic(&self) -> Instant {
        Instant::now()
    }
}

/// Relógio de teste: o tempo só anda quando alguém empurra.
///
/// Mora fora de `#[cfg(test)]` de propósito. Os testes do crate de storage e do
/// desktop precisam dele, e um utilitário de teste que só existe dentro do
/// próprio módulo obriga cada camada a inventar o seu — e aí duas camadas
/// discordam sobre o que "agora" significa.
#[derive(Clone, Debug)]
pub struct FixedClock {
    wall: std::sync::Arc<std::sync::Mutex<OffsetDateTime>>,
    origin: Instant,
    /// Quanto o monotônico avançou. Separado do relógio de parede porque é a
    /// divergência entre os dois que os testes de sono precisam produzir.
    monotonic_offset: std::sync::Arc<std::sync::Mutex<std::time::Duration>>,
}

impl FixedClock {
    pub fn at(moment: OffsetDateTime) -> Self {
        Self {
            wall: std::sync::Arc::new(std::sync::Mutex::new(moment)),
            origin: Instant::now(),
            monotonic_offset: std::sync::Arc::new(std::sync::Mutex::new(
                std::time::Duration::ZERO,
            )),
        }
    }

    /// Avança as duas leituras juntas: tempo passando normalmente.
    pub fn advance(&self, span: time::Duration) {
        *self.wall.lock().unwrap() += span;
        let positive = span.unsigned_abs();
        *self.monotonic_offset.lock().unwrap() += positive;
    }

    /// Move só o relógio de parede.
    ///
    /// É o que acontece num ajuste de relógio, na entrada do horário de verão e
    /// — do ponto de vista de quem observa — na volta de um sono em que o
    /// monotônico do sistema não contou o tempo dormido.
    pub fn skew_wall(&self, span: time::Duration) {
        *self.wall.lock().unwrap() += span;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        *self.wall.lock().unwrap()
    }

    fn monotonic(&self) -> Instant {
        self.origin + *self.monotonic_offset.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn epoch() -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH
    }

    #[test]
    fn a_fixed_clock_does_not_move_by_itself() {
        let clock = FixedClock::at(epoch());
        let first = clock.now();
        std::thread::yield_now();
        assert_eq!(clock.now(), first);
    }

    #[test]
    fn advancing_moves_both_readings() {
        let clock = FixedClock::at(epoch());
        let before = clock.monotonic();

        clock.advance(Duration::hours(2));

        assert_eq!(clock.now(), epoch() + Duration::hours(2));
        assert_eq!(
            clock.monotonic().duration_since(before),
            std::time::Duration::from_secs(7_200)
        );
    }

    /// O caso que o agendador precisa detectar: a parede saltou uma hora e o
    /// monotônico não. Sem essa divergência, "a máquina dormiu" seria
    /// indistinguível de "o tempo passou normalmente".
    #[test]
    fn skewing_the_wall_leaves_the_monotonic_where_it_was() {
        let clock = FixedClock::at(epoch());
        let before = clock.monotonic();

        clock.skew_wall(Duration::hours(1));

        assert_eq!(clock.now(), epoch() + Duration::hours(1));
        assert_eq!(clock.monotonic(), before, "o monotonico nao acompanha salto");
    }

    #[test]
    fn the_wall_can_go_backwards_because_real_clocks_do() {
        let clock = FixedClock::at(epoch() + Duration::hours(5));
        clock.skew_wall(Duration::hours(-3));
        assert_eq!(clock.now(), epoch() + Duration::hours(2));
    }

    #[test]
    fn the_system_clock_moves_forward() {
        let clock = SystemClock;
        let first = clock.monotonic();
        let second = clock.monotonic();
        assert!(second >= first);
    }
}
