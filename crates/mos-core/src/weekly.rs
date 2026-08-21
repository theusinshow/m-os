//! Weekly Review: o fecho da semana sobre as Daily Sessions.

use serde::{Deserialize, Serialize};
use time::Duration;

use crate::{CoreError, Day, ErrorCode};

/// Uma semana civil, identificada pela **data da segunda-feira**.
///
/// # Por que a segunda, e nao o numero ISO
///
/// Numero ISO tem duas armadilhas que a data da segunda simplesmente nao tem:
/// **semanas 53**, e o 1º de janeiro que pertence a semana 52 do ano anterior.
/// Guardar `2026-W01` obrigaria a escolher uma convencao de virada de ano e a
/// acerta-la em todo lugar que compara; guardar `2026-08-17` nao obriga a nada.
///
/// # Por que civil e fixa, e nao sete dias deslizantes
///
/// Um "fecho" de janela deslizante nao fecha nada, e a unicidade precisa de uma
/// chave. E a mesma razao pela qual [`Day`] existe como campo em vez de ser
/// decidido por cada leitor.
///
/// **Esta e a unica copia da regra.** Nada de `date(day, 'weekday 0', '-6
/// days')` em SQL: a semana calculada em dois lugares e como as duas versoes
/// divergem.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Week(Day);

impl Week {
    /// A semana que contem este dia.
    pub fn containing(day: &Day) -> Result<Self, CoreError> {
        let date = day.date()?;
        // `number_days_from_monday` da 0 na segunda e 6 no domingo, entao a
        // segunda nao anda e o domingo volta seis. As duas bordas sao o caso
        // que uma conta ingenua erra.
        let recuo = i64::from(date.weekday().number_days_from_monday());
        Day::from_date(date - Duration::days(recuo)).map(Self)
    }

    /// Le a data de uma segunda-feira. Recusa qualquer outro dia.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        let day = Day::parse(value)?;
        let date = day.date()?;
        if date.weekday().number_days_from_monday() != 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "A semana e identificada pela segunda-feira.",
                false,
            ));
        }
        Ok(Self(day))
    }

    pub fn start(&self) -> &Day {
        &self.0
    }

    pub fn end(&self) -> Result<Day, CoreError> {
        Day::from_date(self.0.date()? + Duration::days(6))
    }

    pub fn previous(&self) -> Result<Self, CoreError> {
        Day::from_date(self.0.date()? - Duration::days(7)).map(Self)
    }

    pub fn next(&self) -> Result<Self, CoreError> {
        Day::from_date(self.0.date()? + Duration::days(7)).map(Self)
    }

    /// As duas bordas entram.
    pub fn contains(&self, day: &Day) -> Result<bool, CoreError> {
        Ok(*day >= *self.start() && *day <= self.end()?)
    }
}

impl std::fmt::Display for Week {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dia(valor: &str) -> Day {
        Day::parse(valor).unwrap()
    }

    #[test]
    fn a_semana_e_a_segunda_que_contem_o_dia() {
        // 2026-08-21 e uma sexta. A semana dela comeca em 17 e termina em 23.
        let semana = Week::containing(&dia("2026-08-21")).unwrap();
        assert_eq!(semana.start().as_str(), "2026-08-17");
        assert_eq!(semana.end().unwrap().as_str(), "2026-08-23");
    }

    #[test]
    fn a_segunda_e_o_domingo_caem_na_mesma_semana() {
        // As duas bordas sao o caso que uma conta de "menos N dias" erra: a
        // segunda nao pode andar para tras, e o domingo nao pode virar a
        // semana seguinte.
        let segunda = Week::containing(&dia("2026-08-17")).unwrap();
        let domingo = Week::containing(&dia("2026-08-23")).unwrap();
        assert_eq!(segunda, domingo);
        assert_eq!(segunda.start().as_str(), "2026-08-17");
    }

    #[test]
    fn a_semana_atravessa_mes_e_ano_sem_numero_iso() {
        // 2026-01-01 e uma quinta, e a semana dela comeca em 2025-12-29. Com
        // numero ISO isto seria "semana 1 de 2026" comecando em 2025, que e a
        // convencao que este desenho recusa por nao precisar dela.
        let virada = Week::containing(&dia("2026-01-01")).unwrap();
        assert_eq!(virada.start().as_str(), "2025-12-29");
        assert_eq!(virada.end().unwrap().as_str(), "2026-01-04");
    }

    #[test]
    fn anterior_e_proxima_andam_sete_dias() {
        let semana = Week::containing(&dia("2026-03-02")).unwrap();
        assert_eq!(semana.previous().unwrap().start().as_str(), "2026-02-23");
        assert_eq!(semana.next().unwrap().start().as_str(), "2026-03-09");
    }

    #[test]
    fn ano_bissexto_nao_quebra_a_conta() {
        // 2028 e bissexto. A semana que contem 29/02 comeca em 28/02.
        let semana = Week::containing(&dia("2028-02-29")).unwrap();
        assert_eq!(semana.start().as_str(), "2028-02-28");
        assert_eq!(semana.next().unwrap().start().as_str(), "2028-03-06");
    }

    #[test]
    fn a_semana_so_aceita_segunda_feira() {
        // Ela e CHAVE: duas representacoes do mesmo intervalo criariam duas
        // linhas para a mesma semana, e o indice unico nao veria a duplicata.
        assert!(Week::parse("2026-08-17").is_ok());
        assert!(Week::parse("2026-08-21").is_err(), "sexta nao e inicio de semana");
        assert!(Week::parse("2026-8-17").is_err(), "sem zero a esquerda e outra chave");
        assert!(Week::parse("").is_err());
    }

    #[test]
    fn contains_responde_pelas_duas_bordas() {
        let semana = Week::containing(&dia("2026-08-19")).unwrap();
        assert!(semana.contains(&dia("2026-08-17")).unwrap());
        assert!(semana.contains(&dia("2026-08-23")).unwrap());
        assert!(!semana.contains(&dia("2026-08-16")).unwrap());
        assert!(!semana.contains(&dia("2026-08-24")).unwrap());
    }

    #[test]
    fn a_semana_atravessa_a_ponte_como_a_data_da_segunda() {
        // O nome vai para o TypeScript e para o banco. Um formato diferente de
        // cada lado faria a tela deixar de reconhecer a semana sem erro de
        // compilacao de nenhum dos dois.
        let semana = Week::containing(&dia("2026-08-19")).unwrap();
        assert_eq!(serde_json::to_string(&semana).unwrap(), "\"2026-08-17\"");
        assert_eq!(
            serde_json::from_str::<Week>("\"2026-08-17\"").unwrap(),
            semana
        );
    }

    #[test]
    fn as_semanas_se_ordenam_no_tempo() {
        // `pending_week` escolhe a mais recente com `max()`. Sem Ord correto,
        // ela escolheria por ordem alfabetica — que por sorte coincide neste
        // formato, e e exatamente o tipo de sorte que quebra quando o formato
        // muda.
        let antes = Week::containing(&dia("2025-12-31")).unwrap();
        let depois = Week::containing(&dia("2026-01-05")).unwrap();
        assert!(antes < depois);
    }
}
