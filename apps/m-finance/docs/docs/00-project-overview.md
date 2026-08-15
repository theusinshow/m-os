# 00 — Project Overview

# M Finance — Visão Geral do Projeto

## Objetivo do Documento

Este documento apresenta a visão geral do projeto **M Finance**.

Ele deve ser lido antes de qualquer decisão de design, arquitetura técnica, banco de dados, modelagem, implementação ou criação de componentes.

A função deste documento é garantir que o projeto comece com uma direção clara e não seja confundido com um aplicativo financeiro genérico.

---

# Identificação do Projeto

## Nome do Produto

**M Finance**

## Nome do Repositório

`m-finance`

## Tipo de Produto

Software web pessoal de planejamento financeiro mensal.

## Usuário Principal

Matheus Mendes.

## Público

Uso individual, privado e exclusivo.

O produto não nasce como SaaS, aplicativo público, plataforma multiusuário ou produto comercial.

A arquitetura pode ser suficientemente organizada para evoluir no futuro, mas a versão inicial deve ser desenhada para uso pessoal.

---

# Definição Curta

O **M Finance** é um cockpit pessoal para controlar contas mensais, vencimentos, faturas simples, receitas previstas, histórico básico e poder de compra.

Ele existe para ajudar Matheus a entender o mês financeiro atual e tomar decisões melhores sobre novas compras.

---

# Definição Expandida

O M Finance é um sistema web que organiza o mês financeiro em torno de compromissos reais:

- contas fixas;
- contas variáveis;
- contas únicas;
- faturas de cartões;
- receitas previstas;
- receitas extras;
- freelancers;
- vencimentos;
- contas pagas;
- contas pendentes;
- contas vencidas;
- histórico mensal simples;
- simulações futuras;
- metas separadas.

O produto deve apresentar essas informações em uma interface visualmente refinada, direta e funcional, seguindo uma estética financeira limpa com influência da identidade visual da Coded by M.

---

# Problema Principal

Matheus usa o Mobills principalmente para lembrar, no início do mês, tudo que precisa pagar.

O problema não é falta de um app financeiro completo.

O problema é:

- depender de um app pago;
- usar uma ferramenta genérica;
- ter funções demais que não são necessárias;
- não ter uma experiência visual própria;
- não ter exatamente a lógica mental desejada;
- não ter uma tela feita para responder rapidamente se o mês está controlado;
- não ter um simulador de compra futura alinhado ao próprio uso.

---

# Solução Proposta

Criar um software web privado, com login via Google, para controlar o mês financeiro de forma objetiva.

O sistema deve permitir que Matheus:

1. veja o resumo do mês;
2. cadastre receitas previstas;
3. cadastre contas mensais;
4. defina se uma conta é recorrente ou única;
5. revise a geração de um novo mês;
6. use valores anteriores como sugestão;
7. marque contas como pagas rapidamente;
8. visualize vencimentos no calendário;
9. controle faturas simples de cartão;
10. receba alertas internos;
11. guarde histórico simples;
12. simule compras futuras;
13. acompanhe metas separadamente.

---

# Frase-Guia

> O M Finance mostra o que ainda pode ser feito com segurança, não apenas o que já foi gasto.

Essa frase deve guiar todas as decisões do produto.

---

# O Que o M Finance É

O M Finance é:

- um cockpit mensal;
- um sistema de vencimentos;
- um painel de contas;
- um controle simples de faturas;
- uma ferramenta de decisão de compra;
- um sistema pessoal privado;
- um app para organizar o mês;
- um produto com identidade visual própria.

---

# O Que o M Finance Não É

O M Finance não é:

- um clone completo do Mobills;
- um aplicativo de transações detalhadas;
- um substituto de banco;
- uma ferramenta de conciliação bancária;
- uma plataforma contábil;
- um ERP pessoal;
- um app de investimentos avançado;
- um sistema multiusuário público;
- uma planilha disfarçada;
- um dashboard genérico sem personalidade.

---

# Decisões Já Fechadas

## Produto

- Nome: M Finance.
- Repositório: `m-finance`.
- Uso: pessoal e exclusivo.
- Usuário: Matheus Mendes.
- Categoria: cockpit pessoal de planejamento financeiro mensal.

## Escopo

- Não registrar toda compra do dia a dia.
- Priorizar contas, vencimentos, faturas e visão mensal.
- Usar receitas fixas variáveis + freelancers.
- Permitir contas recorrentes e únicas.
- Marcar conta como paga com uma ação simples.
- Usar fatura simples de cartão.
- Ter simulador de compra futura.
- Ter metas como acompanhamento separado.
- Ter histórico simples.

## Stack

- Next.js.
- TypeScript.
- Tailwind CSS.
- Supabase.
- PostgreSQL.
- Supabase Auth com Google.
- Drizzle ORM.
- Zod.
- React Hook Form.
- Recharts.

## Experiência

- Desktop como prioridade de construção inicial.
- Mobile deve funcionar bem.
- Visual: financeiro limpo + Coded by M.
- Dashboard em estilo cockpit.
- Exportação fora do MVP 1.

---

# Prioridade do MVP 1

O MVP 1 deve substituir o uso atual do Mobills.

O uso principal é:

> abrir o app, ver o mês, lembrar o que precisa pagar e marcar contas como pagas.

Se uma funcionalidade não ajuda esse fluxo, ela deve ser adiada.

---

# Métrica de Sucesso do MVP 1

O MVP 1 será considerado bem-sucedido se Matheus conseguir:

- cadastrar suas contas mensais;
- abrir o dashboard e entender o mês atual em poucos segundos;
- ver vencimentos próximos;
- marcar contas como pagas rapidamente;
- controlar faturas dos cartões principais;
- revisar a criação do próximo mês;
- abandonar o Mobills para esse uso principal.

---

# Princípios de Produto

## 1. Clareza acima de completude

O sistema não precisa mostrar tudo.

Ele precisa mostrar o que importa para decidir.

## 2. Pouca fricção

A ação mais usada será marcar uma conta como paga.

Essa ação deve ser extremamente rápida.

## 3. Planejamento acima de rastreamento

O sistema deve olhar para frente.

Ele deve ajudar a prever vencimentos, compromissos e impacto de compras.

## 4. Visual próprio sem sacrificar função

O app deve ter identidade visual forte, mas não pode virar uma experiência confusa.

## 5. Escopo protegido

Não transformar o M Finance em um sistema financeiro genérico.

---

# Principais Módulos

1. Autenticação.
2. Dashboard.
3. Receitas.
4. Contas.
5. Cartões.
6. Faturas.
7. Calendário.
8. Alertas.
9. Histórico.
10. Configurações.
11. Simulador.
12. Metas.

---

# Versões do Produto

## MVP 1 — Controle Mensal Real

Substituir o uso atual do Mobills.

## MVP 2 — Decisão de Compra

Adicionar simulador, metas e projeções futuras.

## MVP 3 — Experiência Premium

Adicionar PWA, notificações, exportação, refinamento visual e histórico avançado.

---

# Regra Final

Sempre que houver dúvida entre criar uma funcionalidade financeira completa ou manter o produto simples e útil:

**manter simples e útil.**

Sempre que houver dúvida entre visual impressionante e clareza de uso:

**clareza de uso vence.**
