/**
 * Sonda de investigacao do Univirtus — SOMENTE LEITURA.
 *
 * Como usar: autentique no AVA pelo navegador normalmente, abra o console em
 * qualquer pagina de https://univirtus.uninter.com/ava/web/ e cole este arquivo.
 *
 * Ela NAO contem segredo nenhum: reusa a sessao viva da aba, chamando `$.ajax`,
 * que e quem anexa o cabecalho `X-time` da sessao. Nada e gravado em disco e
 * nenhum valor de sessao e impresso.
 *
 * Todas as chamadas sao GET. Nenhuma delas inicia avaliacao, entrega trabalho
 * ou altera estado — ver docs/UNIVIRTUS-INTEGRATION.md, secao "O que nao tocar".
 */
(async () => {
  const $ = window.jQuery;
  const get = (u) => new Promise((ok) =>
    $.ajax({ url: u, method: 'GET' }).done(ok).fail((x) => ok({ __status: x.status })));

  const log = (...a) => console.log('[univirtus]', ...a);

  // 1. Sessao viva?
  const escola = await get('/ava/sistema/Escola/0/Usuario');
  if (escola.__status) { log('sessao invalida (HTTP ' + escola.__status + ')'); return; }
  log('sessao valida');

  // 2. Disciplinas do aluno (todas as ofertas em que ele esta inscrito).
  //    codigoOferta === 0 sao salas de apoio (Estagio, Pesquisa e extensao),
  //    e nao disciplinas: elas nao tem contrapartida no historico.
  const hist = await get('/ava/sistema/UsuarioHistoricoCursoOferta/false/Usuario/');
  const ofertas = (hist.usuarioHistoricoCursoOfertas || []).filter((o) => o.codigoOferta !== 0);
  log(ofertas.length + ' disciplinas encontradas');

  // 3. Historico academico: e ele quem diz o SEMESTRE e a nota final.
  const cursos = await get('/ava/sistema/UsuarioCurso/0/GetCursosAproveitamento?idUsuario=0');
  const curso = (cursos.cursosAproveitamento || [])[0];
  if (!curso) { log('nenhum curso'); return; }
  log('curso: ' + curso.curso.nome + ' (idCurso ' + curso.idCurso + ')');

  const hist2 = await get('/ava/integracao/UsuarioIntegracaoSistemaAcademico/0/GetDisciplinasAproveitamento'
    + '?sidCdAluno=' + encodeURIComponent(curso.sCdAluno));
  const porOferta = Object.fromEntries((hist2.aproveitamento || []).map((a) => [a.cdOfertaDisciplina, a]));
  const semestres = [...new Set(Object.values(porOferta).map((a) => a.nomeModuloPOrdenacao))].sort();
  log('semestres: ' + semestres.join(', '));

  // 4. O semestre corrente e o maior codigo com disciplina "EM CURSO".
  const corrente = semestres[semestres.length - 1];
  const atuais = ofertas.filter((o) => porOferta[o.codigoOferta]?.nomeModuloPOrdenacao === corrente);
  log(atuais.length + ' disciplinas no semestre corrente (' + corrente + ')');

  // 5. Avaliacoes e trabalhos, por disciplina. Nao ha endpoint consolidado.
  let totalAv = 0, totalTr = 0, pendentes = 0;
  for (const o of atuais) {
    const av = await get('/ava/bqs/AvaliacaoUsuario/1/paginacao/true?numRegistros=100&filtro=&ordenacao='
      + '&idSalaVirtual=' + o.idSalaVirtual
      + '&idSalaVirtualOferta=' + o.idSalaVirtualOferta
      + '&ajustarDatasMatriculaCurso=false');
    const tr = await get('/ava/interacao/TrabalhoEtapa/' + o.idSalaVirtualOferta
      + '/GetEtapasByOfertaInscrito/false?master=true&idSalaVirtualOfertaAproveitamento=' + o.idSalaVirtualOferta);
    const avs = av.avaliacaoUsuarios || [];
    const trs = tr.trabalhoEtapas || [];
    totalAv += avs.length; totalTr += trs.length;
    pendentes += avs.filter((x) => x.nota === null).length + trs.filter((x) => x.dataEntrega === null).length;
    log('  ' + o.nomeSalaVirtual + ': ' + avs.length + ' avaliacoes, ' + trs.length + ' trabalhos');
  }
  log(totalAv + ' avaliacoes, ' + totalTr + ' trabalhos, ' + pendentes + ' sem nota/entrega');

  // 6. Roteiro de estudo e material de uma disciplina, so para provar o caminho.
  const primeira = atuais[0];
  if (primeira) {
    const est = await get('/ava/ava/SalaVirtualEstrutura/' + primeira.idSalaVirtual + '/TipoOferta/1'
      + '?idSalaVirtualOferta=' + primeira.idSalaVirtualOferta
      + '&idSalaVirtualOfertaAproveitamento=&idSalaVirtualOfertaPai=');
    const aulas = est.salaVirtualEstruturas || [];
    log(aulas.length + ' aulas em ' + primeira.nomeSalaVirtual);
    const comAtividade = aulas.find((a) => a.totalAtividades > 0);
    if (comAtividade) {
      const ats = await get('/ava/ava/salaVirtualAtividade/0/EstruturaOferta/' + primeira.idSalaVirtualOferta
        + '/?id=' + comAtividade.id + '&editar=false&idSalaVirtualOfertaPai=&idSalaVirtualOfertaAproveitamento=');
      const lista = ats.salaVirtualAtividades || [];
      log('  ' + comAtividade.estrutura + ': ' + lista.length + ' atividades');
      // Nem toda atividade tem arquivo: videoaula e leitura sem anexo devolvem
      // lista vazia. Varre ate achar uma que tenha, senao o zero mentiria.
      let mats = [];
      for (const at of lista) {
        const itens = await get('/ava/atv/AtividadeItemAprendizagem/' + at.idAtividade + '/Atividade?complementar=false');
        mats = (itens.atividadeItemAprendizagens || [])
          .flatMap((i) => (i.itemAprendizagemEtiquetas || []))
          .map((e) => e.sistemaRepositorio).filter(Boolean);
        if (mats.length) { log('  material em "' + at.nomeAtividade + '"'); break; }
      }
      log('  ' + mats.length + ' materiais no roteiro (ids: ' + mats.map((m) => m.id).join(', ') + ')');
    }

    // Material complementar e OUTRA estrutura (TipoOferta/2), e e onde mora o
    // Plano de Ensino. Quem so varrer o roteiro nao acha nenhum dos dois.
    const comp = await get('/ava/ava/SalaVirtualEstrutura/' + primeira.idSalaVirtual + '/TipoOferta/2/'
      + '?idSalaVirtualOferta=' + primeira.idSalaVirtualOferta
      + '&idSalaVirtualOfertaPai=null&idSalaVirtualOfertaAproveitamento=null');
    let compMats = [];
    for (const sec of (comp.salaVirtualEstruturas || [])) {
      const ats = await get('/ava/ava/SalaVirtualAtividade/' + sec.id + '/EstruturaOferta/' + primeira.idSalaVirtualOferta
        + '?idSalaVirtualOfertaPai=null&idSalaVirtualOfertaAproveitamento=null'
        + '&buscarItemAprendizagem=true&ocultarAtividadeSemItem=true');
      for (const at of (ats.salaVirtualAtividades || [])) {
        const itens = await get('/ava/atv/AtividadeItemAprendizagem/' + at.idAtividade + '/Atividade?complementar=true');
        compMats.push(...(itens.atividadeItemAprendizagens || [])
          .flatMap((i) => (i.itemAprendizagemEtiquetas || []))
          .map((e) => e.sistemaRepositorio).filter(Boolean));
      }
    }
    log('  ' + compMats.length + ' materiais complementares (ids: ' + compMats.map((m) => m.id).join(', ') + ')');
  }
  log('fim — nenhuma escrita foi feita');
})();
