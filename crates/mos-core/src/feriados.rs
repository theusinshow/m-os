//! Os feriados, calculados e não tabelados.
//!
//! # Por que não uma tabela de datas
//!
//! Uma lista de "2026-01-01, 2026-02-17, …" precisa ser reescrita todo ano, e o
//! ano em que ninguém reescrever é o ano em que o calendário fica em branco sem
//! avisar. Os feriados brasileiros são REGRA: nove datas fixas e três que
//! dependem da Páscoa, que por sua vez é uma conta fechada desde 1583.
//!
//! Então aqui não há dado — há a regra. Ela vale para qualquer ano, inclusive os
//! que ninguém pensou em preencher.
//!
//! # O que este módulo NÃO faz
//!
//! Feriado estadual e municipal. Eles não são regra: são lei local, mudam por
//! decreto e não se derivam de nada. Cobri-los exige uma FONTE — um arquivo que
//! alguém mantém, ou uma API —, e inventar uma tabela de Santa Catarina aqui
//! seria exatamente o hardcode que este módulo existe para evitar.
//!
//! O tipo já carrega o `escopo` para o dia em que essa fonte existir.

use serde::{Deserialize, Serialize};
use time::{Date, Month};

/// De quem é o feriado.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscopoDoFeriado {
    Nacional,
    Estadual,
    Municipal,
}

/// O que a lei diz sobre o dia.
///
/// A distinção importa e é frequentemente ignorada: Carnaval e Corpus Christi
/// **não são feriados nacionais** — são ponto facultativo, e cada empregador
/// decide. Um calendário que os pintasse iguais ao Natal mentiria sobre um dia
/// em que muita gente trabalha.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PesoDoFeriado {
    /// Feriado por lei: não se trabalha.
    Feriado,
    /// Ponto facultativo: costume forte, obrigação nenhuma.
    Facultativo,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feriado {
    pub data: Date,
    pub nome: String,
    pub escopo: EscopoDoFeriado,
    pub peso: PesoDoFeriado,
}

/// O domingo de Páscoa do ano, pelo algoritmo gregoriano anônimo.
///
/// É a conta de que dependem Carnaval, Sexta-feira Santa e Corpus Christi. O
/// algoritmo é o de Meeus/Jones/Butcher, e a razão de ele estar aqui em vez de
/// numa dependência é que ele tem doze linhas e não muda desde 1583 — uma caixa
/// de dependência para isso custaria mais em auditoria do que em código.
pub fn pascoa(ano: i32) -> Date {
    let a = ano % 19;
    let b = ano / 100;
    let c = ano % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let mes = (h + l - 7 * m + 114) / 31;
    let dia = ((h + l - 7 * m + 114) % 31) + 1;
    Date::from_calendar_date(
        ano,
        Month::try_from(mes as u8).expect("o algoritmo só produz março ou abril"),
        dia as u8,
    )
    .expect("o algoritmo só produz datas válidas")
}

/// Os feriados nacionais de um ano, do primeiro ao último.
pub fn nacionais(ano: i32) -> Vec<Feriado> {
    let fixo = |mes: Month, dia: u8, nome: &str| Feriado {
        data: Date::from_calendar_date(ano, mes, dia).expect("data fixa é sempre válida"),
        nome: nome.to_owned(),
        escopo: EscopoDoFeriado::Nacional,
        peso: PesoDoFeriado::Feriado,
    };

    let pascoa = pascoa(ano);
    let movel = |dias: i64, nome: &str, peso: PesoDoFeriado| Feriado {
        data: pascoa + time::Duration::days(dias),
        nome: nome.to_owned(),
        escopo: EscopoDoFeriado::Nacional,
        peso,
    };

    let mut dias = vec![
        fixo(Month::January, 1, "Confraternização Universal"),
        // Carnaval é segunda e terça, e as duas entram: quem marca uma viagem
        // olha o bloco, e um calendário que mostrasse só a terça faria a segunda
        // parecer dia útil.
        movel(-48, "Carnaval", PesoDoFeriado::Facultativo),
        movel(-47, "Carnaval", PesoDoFeriado::Facultativo),
        movel(-2, "Sexta-feira Santa", PesoDoFeriado::Feriado),
        movel(0, "Páscoa", PesoDoFeriado::Facultativo),
        fixo(Month::April, 21, "Tiradentes"),
        fixo(Month::May, 1, "Dia do Trabalho"),
        movel(60, "Corpus Christi", PesoDoFeriado::Facultativo),
        fixo(Month::September, 7, "Independência"),
        fixo(Month::October, 12, "Nossa Senhora Aparecida"),
        fixo(Month::November, 2, "Finados"),
        fixo(Month::November, 15, "Proclamação da República"),
        // Nacional desde a Lei 14.759/2023. Antes disso era estadual em parte do
        // país — e por isso a data só entra a partir de 2024: pintá-la como
        // feriado nacional em 2020 seria o calendário inventando história.
        fixo(Month::November, 20, "Consciência Negra"),
        fixo(Month::December, 25, "Natal"),
    ];
    if ano < 2024 {
        dias.retain(|feriado| feriado.nome != "Consciência Negra");
    }
    dias.sort_by_key(|feriado| feriado.data);
    dias
}

/// Os feriados nacionais que caem entre duas datas, inclusive.
///
/// Atravessa a virada do ano sozinho: uma janela de dezembro a janeiro precisa
/// dos dois anos, e quem chama não deveria ter que saber disso.
pub fn nacionais_entre(inicio: Date, fim: Date) -> Vec<Feriado> {
    let mut dias = Vec::new();
    for ano in inicio.year()..=fim.year() {
        dias.extend(
            nacionais(ano)
                .into_iter()
                .filter(|feriado| feriado.data >= inicio && feriado.data <= fim),
        );
    }
    dias.sort_by_key(|feriado| feriado.data);
    dias
}

#[cfg(test)]
mod testes {
    use super::*;

    fn dia(ano: i32, mes: u8, dia: u8) -> Date {
        Date::from_calendar_date(ano, Month::try_from(mes).unwrap(), dia).unwrap()
    }

    /// Datas conferidas contra o calendário litúrgico, e não contra a própria
    /// implementação: um teste que só repete o que o código faz não descobre
    /// nada.
    #[test]
    fn a_pascoa_bate_com_os_anos_conhecidos() {
        assert_eq!(pascoa(2024), dia(2024, 3, 31));
        assert_eq!(pascoa(2025), dia(2025, 4, 20));
        assert_eq!(pascoa(2026), dia(2026, 4, 5));
        assert_eq!(pascoa(2027), dia(2027, 3, 28));
        assert_eq!(pascoa(2030), dia(2030, 4, 21));
        // Um ano bem longe, para o algoritmo não estar acertando por acaso numa
        // vizinhança.
        assert_eq!(pascoa(2100), dia(2100, 3, 28));
    }

    #[test]
    fn o_carnaval_e_a_sexta_santa_seguem_a_pascoa() {
        let dias = nacionais(2026);
        let carnaval: Vec<_> = dias.iter().filter(|f| f.nome == "Carnaval").collect();
        assert_eq!(carnaval.len(), 2, "carnaval e segunda E terca");
        assert_eq!(carnaval[0].data, dia(2026, 2, 16));
        assert_eq!(carnaval[1].data, dia(2026, 2, 17));

        let santa = dias.iter().find(|f| f.nome == "Sexta-feira Santa").unwrap();
        assert_eq!(santa.data, dia(2026, 4, 3));

        let corpus = dias.iter().find(|f| f.nome == "Corpus Christi").unwrap();
        assert_eq!(corpus.data, dia(2026, 6, 4));
    }

    /// Carnaval e Corpus Christi não são feriado por lei, e pintá-los iguais ao
    /// Natal mentiria sobre um dia em que muita gente trabalha.
    #[test]
    fn o_facultativo_nao_se_confunde_com_feriado() {
        let dias = nacionais(2026);
        let peso = |nome: &str| dias.iter().find(|f| f.nome == nome).unwrap().peso;
        assert_eq!(peso("Carnaval"), PesoDoFeriado::Facultativo);
        assert_eq!(peso("Corpus Christi"), PesoDoFeriado::Facultativo);
        assert_eq!(peso("Natal"), PesoDoFeriado::Feriado);
        assert_eq!(peso("Sexta-feira Santa"), PesoDoFeriado::Feriado);
    }

    /// A Consciência Negra virou nacional pela Lei 14.759/2023. Antes disso ela
    /// era estadual em parte do país, e pintá-la como nacional em 2020 seria o
    /// calendário inventando história.
    #[test]
    fn a_consciencia_negra_so_e_nacional_a_partir_de_2024() {
        assert!(nacionais(2023)
            .iter()
            .all(|f| f.nome != "Consciência Negra"));
        assert!(nacionais(2024)
            .iter()
            .any(|f| f.nome == "Consciência Negra"));
    }

    #[test]
    fn a_janela_atravessa_a_virada_do_ano() {
        let dias = nacionais_entre(dia(2025, 12, 20), dia(2026, 1, 10));
        let nomes: Vec<_> = dias.iter().map(|f| f.nome.as_str()).collect();
        assert_eq!(nomes, vec!["Natal", "Confraternização Universal"]);
    }

    #[test]
    fn a_janela_de_um_dia_pega_o_feriado_daquele_dia() {
        let natal = nacionais_entre(dia(2026, 12, 25), dia(2026, 12, 25));
        assert_eq!(natal.len(), 1);
        assert_eq!(natal[0].nome, "Natal");
    }
}
