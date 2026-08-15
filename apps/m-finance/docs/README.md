# M Finance — Documentação do Produto

Este pacote contém a documentação inicial completa do **M Finance**, um software web pessoal de planejamento financeiro mensal criado para uso exclusivo de Matheus Mendes.

O objetivo destes documentos é orientar a criação do projeto no Claude Code, Codex, Cursor ou qualquer outro ambiente assistido por IA, evitando desvio de escopo e mantendo o produto coerente com a proposta definida.

---

## Produto

**Nome:** M Finance  
**Repositório sugerido:** `m-finance`  
**Tipo:** software web pessoal  
**Usuário:** Matheus Mendes  
**Categoria:** cockpit pessoal de contas, vencimentos, faturas e poder de compra  

---

## Frase-guia

> O M Finance mostra o que ainda pode ser feito com segurança, não apenas o que já foi gasto.

---

## Documentos

1. `00-project-overview.md`
2. `01-product-vision.md`
3. `02-mvp-scope.md`
4. `03-functional-requirements.md`
5. `04-information-architecture.md`
6. `05-user-flows.md`
7. `06-data-model.md`
8. `07-business-rules.md`
9. `08-technical-architecture.md`
10. `09-ui-ux-direction.md`
11. `10-development-roadmap.md`
12. `11-ai-implementation-prompt.md`
13. `CLAUDE.md`
14. `codex.md`

---

## Uso recomendado

Antes de implementar qualquer coisa, peça para a IA ler todos os arquivos da pasta `/docs`.

Prompt base:

```txt
Leia todos os arquivos da pasta /docs antes de responder ou implementar.
Não implemente funcionalidades fora do escopo definido.
O MVP 1 deve substituir o uso atual do Mobills: lembrar contas, mostrar vencimentos, controlar faturas simples e permitir marcar contas como pagas.
Não transforme o produto em um app genérico de transações financeiras.
```

---

## Escopo principal

O M Finance não é um clone completo do Mobills.

Ele não deve priorizar:

- registro de todas as compras;
- integração bancária;
- conciliação de extrato;
- importação automática;
- relatório financeiro empresarial;
- gestão contábil;
- controle completo de investimentos.

Ele deve priorizar:

- clareza mensal;
- contas a pagar;
- vencimentos;
- faturas simples;
- receitas previstas;
- alertas;
- visão de sobra estimada;
- decisão de compra futura.

---

## Stack recomendada

- Next.js
- TypeScript
- Tailwind CSS
- Supabase
- PostgreSQL
- Supabase Auth com Google
- Drizzle ORM
- Zod
- React Hook Form
- Recharts

---

## Ordem recomendada de leitura

1. Visão do produto
2. Escopo do MVP
3. Requisitos funcionais
4. Arquitetura de informação
5. Regras de negócio
6. Modelo de dados
7. Arquitetura técnica
8. UI/UX
9. Roadmap
10. Prompt de implementação
