import { memo, type ReactNode } from "react";
import { api } from "./api";

/**
 * Markdown como elementos React. Nunca como HTML.
 *
 * O perigo de renderizar conteudo vindo de fora nao esta em interpretar
 * Markdown: esta em produzir HTML e depois tentar limpa-lo. Este renderer nao
 * tem sanitizador porque nao tem o que sanitizar — o React escapa texto por
 * construcao, e sem `dangerouslySetInnerHTML` nao existe caminho de injecao.
 * Uma biblioteca madura e segura pelo mesmo motivo, nao por um motivo melhor.
 * Ver ADR-026.
 *
 * Duas coisas que sao consequencia do chat e nao do Markdown:
 *
 * 1. cerca aberta durante o streaming e tratada como bloco de codigo ABERTO, e
 *    nao como texto solto. Sem isso o bloco piscaria a cada token ate a cerca
 *    fechar;
 * 2. o que estiver fora do subconjunto aparece como texto, nunca como marcacao
 *    quebrada.
 */

type Block =
  | { kind: "heading"; level: number; text: string }
  | { kind: "paragraph"; text: string }
  | { kind: "code"; language: string; code: string; open: boolean }
  | { kind: "quote"; lines: string[] }
  | { kind: "list"; ordered: boolean; items: { text: string; depth: number }[] }
  | { kind: "table"; header: string[]; rows: string[][] }
  | { kind: "rule" };

const FENCE = /^\s*(`{3,}|~{3,})\s*([\w+-]*)\s*$/;
const HEADING = /^(#{1,6})\s+(.*)$/;
const RULE = /^\s*(-{3,}|\*{3,}|_{3,})\s*$/;
const QUOTE = /^\s*>\s?(.*)$/;
const UNORDERED = /^(\s*)[-*+]\s+(.*)$/;
const ORDERED = /^(\s*)\d+[.)]\s+(.*)$/;
const TABLE_DIVIDER = /^\s*\|?[\s:|-]+\|[\s:|-]*$/;

function splitRow(line: string): string[] {
  return line
    .replace(/^\s*\|/, "")
    .replace(/\|\s*$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

export function parseBlocks(source: string): Block[] {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];

    const fence = FENCE.exec(line);
    if (fence) {
      const marker = fence[1][0];
      const code: string[] = [];
      index += 1;
      let closed = false;
      while (index < lines.length) {
        const candidate = FENCE.exec(lines[index]);
        if (candidate && candidate[1][0] === marker) {
          closed = true;
          index += 1;
          break;
        }
        code.push(lines[index]);
        index += 1;
      }
      blocks.push({ kind: "code", language: fence[2] ?? "", code: code.join("\n"), open: !closed });
      continue;
    }

    if (!line.trim()) {
      index += 1;
      continue;
    }

    if (RULE.test(line)) {
      blocks.push({ kind: "rule" });
      index += 1;
      continue;
    }

    const heading = HEADING.exec(line);
    if (heading) {
      blocks.push({ kind: "heading", level: heading[1].length, text: heading[2] });
      index += 1;
      continue;
    }

    if (QUOTE.test(line)) {
      const quoted: string[] = [];
      while (index < lines.length && QUOTE.test(lines[index])) {
        quoted.push(QUOTE.exec(lines[index])![1]);
        index += 1;
      }
      blocks.push({ kind: "quote", lines: quoted });
      continue;
    }

    // Tabela: so vale com a linha divisoria logo abaixo do cabecalho. Sem essa
    // exigencia, qualquer frase com uma barra viraria tabela de uma coluna.
    if (line.includes("|") && index + 1 < lines.length && TABLE_DIVIDER.test(lines[index + 1])) {
      const header = splitRow(line);
      index += 2;
      const rows: string[][] = [];
      while (index < lines.length && lines[index].includes("|") && lines[index].trim()) {
        rows.push(splitRow(lines[index]));
        index += 1;
      }
      blocks.push({ kind: "table", header, rows });
      continue;
    }

    const bullet = UNORDERED.exec(line);
    const numbered = ORDERED.exec(line);
    if (bullet || numbered) {
      const ordered = Boolean(numbered);
      const items: { text: string; depth: number }[] = [];
      while (index < lines.length) {
        const nextBullet = UNORDERED.exec(lines[index]);
        const nextNumbered = ORDERED.exec(lines[index]);
        const match = ordered ? nextNumbered : nextBullet;
        if (!match) break;
        items.push({ text: match[2], depth: Math.min(Math.floor(match[1].length / 2), 3) });
        index += 1;
      }
      blocks.push({ kind: "list", ordered, items });
      continue;
    }

    // Paragrafo: junta ate a linha em branco ou o proximo bloco reconhecivel.
    const paragraph: string[] = [];
    while (index < lines.length) {
      const candidate = lines[index];
      if (
        !candidate.trim() ||
        FENCE.test(candidate) ||
        HEADING.test(candidate) ||
        RULE.test(candidate) ||
        QUOTE.test(candidate) ||
        UNORDERED.test(candidate) ||
        ORDERED.test(candidate)
      ) {
        break;
      }
      paragraph.push(candidate);
      index += 1;
    }
    blocks.push({ kind: "paragraph", text: paragraph.join("\n") });
  }

  return blocks;
}

const INLINE = /(`[^`]+`)|(\*\*[^*]+\*\*)|(~~[^~]+~~)|(\*[^*\n]+\*)|(_[^_\n]+_)|(\[[^\]]*\]\([^)\s]+\))|(https?:\/\/[^\s<>()]+)/g;

/**
 * Abre um link no navegador do sistema, nunca dentro do WebView.
 *
 * Mesma regra que `open_resource` ja segue: navegacao dentro da janela
 * transformaria o M/OS num navegador com privilegio de aplicacao. O backend
 * recusa o que nao for http(s) — e este alvo foi escrito por um modelo.
 */
function openExternally(url: string) {
  void api.openExternalUrl(url).catch(() => undefined);
}

export function renderInline(text: string, keyPrefix = ""): ReactNode[] {
  const nodes: ReactNode[] = [];
  let cursor = 0;
  let match: RegExpExecArray | null;
  INLINE.lastIndex = 0;

  while ((match = INLINE.exec(text)) !== null) {
    if (match.index > cursor) nodes.push(text.slice(cursor, match.index));
    const token = match[0];
    const key = `${keyPrefix}-${match.index}`;

    if (token.startsWith("`")) {
      nodes.push(<code className="md-code" key={key}>{token.slice(1, -1)}</code>);
    } else if (token.startsWith("**")) {
      nodes.push(<strong key={key}>{token.slice(2, -2)}</strong>);
    } else if (token.startsWith("~~")) {
      nodes.push(<s key={key}>{token.slice(2, -2)}</s>);
    } else if (token.startsWith("*") || token.startsWith("_")) {
      nodes.push(<em key={key}>{token.slice(1, -1)}</em>);
    } else if (token.startsWith("[")) {
      const label = token.slice(1, token.indexOf("]"));
      const url = token.slice(token.indexOf("](") + 2, -1);
      nodes.push(
        <a className="md-link" key={key} href={url} onClick={(event) => { event.preventDefault(); openExternally(url); }}>
          {label || url}
        </a>,
      );
    } else {
      nodes.push(
        <a className="md-link" key={key} href={token} onClick={(event) => { event.preventDefault(); openExternally(token); }}>
          {token}
        </a>,
      );
    }
    cursor = match.index + token.length;
  }

  if (cursor < text.length) nodes.push(text.slice(cursor));
  return nodes;
}

/**
 * Realce de sintaxe com tres classes: comentario, literal e palavra-chave.
 *
 * Nao e economia de esforco, e coerencia: `UX-PRINCIPLES.md` §48 restringe cor a
 * funcao, e um tema de dez cores dentro de uma interface que mantem a primaria
 * abaixo de 10% seria o elemento mais colorido do produto inteiro.
 */
const KEYWORDS: Record<string, string> = {
  rust: "as async await break const continue crate dyn else enum extern fn for if impl in let loop match mod move mut pub ref return self static struct super trait type unsafe use where while",
  ts: "abstract any as async await boolean break case catch class const continue declare default delete do else enum export extends false finally for from function if implements import in instanceof interface let new null number private protected public readonly return static string super switch this throw true try type typeof undefined var void while yield",
  js: "async await break case catch class const continue default delete do else export extends false finally for from function if import in instanceof let new null return static super switch this throw true try typeof undefined var void while yield",
  python: "and as assert async await break class continue def del elif else except False finally for from global if import in is lambda None nonlocal not or pass raise return True try while with yield",
  sql: "select from where insert into values update set delete create table index view join left right inner outer on group by order having limit offset primary key foreign references not null unique begin commit",
  shell: "if then else elif fi for while do done case esac function return export local echo cd",
  css: "important media supports keyframes from to and not or",
  json: "true false null",
};

const LANGUAGE_ALIASES: Record<string, keyof typeof KEYWORDS> = {
  rs: "rust",
  rust: "rust",
  ts: "ts",
  tsx: "ts",
  typescript: "ts",
  js: "js",
  jsx: "js",
  javascript: "js",
  py: "python",
  python: "python",
  sql: "sql",
  sh: "shell",
  bash: "shell",
  shell: "shell",
  ps1: "shell",
  powershell: "shell",
  css: "css",
  json: "json",
  toml: "ts",
  yaml: "ts",
  yml: "ts",
};

const TOKENIZER =
  /(\/\/[^\n]*|#[^\n]*|--[^\n]*|\/\*[\s\S]*?\*\/)|("(?:[^"\\\n]|\\.)*"|'(?:[^'\\\n]|\\.)*'|`(?:[^`\\]|\\.)*`)|(\b\d[\d_]*(?:\.\d+)?\b)|([A-Za-z_][A-Za-z0-9_]*)/g;

export function highlight(code: string, language: string): ReactNode[] {
  const family = LANGUAGE_ALIASES[language.toLowerCase()];
  const keywords = family ? new Set(KEYWORDS[family].split(/\s+/)) : null;
  const nodes: ReactNode[] = [];
  let cursor = 0;
  let match: RegExpExecArray | null;
  TOKENIZER.lastIndex = 0;

  while ((match = TOKENIZER.exec(code)) !== null) {
    if (match.index > cursor) nodes.push(code.slice(cursor, match.index));
    const token = match[0];
    const key = `t${match.index}`;

    if (match[1]) nodes.push(<span className="hl-comment" key={key}>{token}</span>);
    else if (match[2] || match[3]) nodes.push(<span className="hl-literal" key={key}>{token}</span>);
    else if (keywords?.has(token)) nodes.push(<span className="hl-keyword" key={key}>{token}</span>);
    else nodes.push(token);

    cursor = match.index + token.length;
  }

  if (cursor < code.length) nodes.push(code.slice(cursor));
  return nodes;
}

function CodeBlock({ language, code, open }: { language: string; code: string; open: boolean }) {
  return (
    <div className="md-codeblock">
      <div className="md-codeblock-head">
        <span className="micro-label">{language || "TEXTO"}</span>
        <button
          type="button"
          className="filter-label"
          onClick={() => void navigator.clipboard.writeText(code)}
          // Um bloco ainda aberto esta incompleto. Copiar metade de um comando e
          // pior que nao oferecer copiar.
          disabled={open}
          title={open ? "O bloco ainda está sendo escrito." : "Copiar código"}
        >
          COPIAR
        </button>
      </div>
      <pre className="md-pre">
        <code>{highlight(code, language)}</code>
      </pre>
    </div>
  );
}

function renderBlock(block: Block, index: number): ReactNode {
  switch (block.kind) {
    case "heading": {
      const Tag = (`h${Math.min(block.level + 2, 6)}`) as "h3" | "h4" | "h5" | "h6";
      return <Tag className="md-heading" key={index}>{renderInline(block.text, `h${index}`)}</Tag>;
    }
    case "paragraph":
      return <p className="md-paragraph" key={index}>{renderInline(block.text, `p${index}`)}</p>;
    case "code":
      return <CodeBlock key={index} language={block.language} code={block.code} open={block.open} />;
    case "quote":
      return (
        <blockquote className="md-quote" key={index}>
          {block.lines.map((line, position) => (
            <p key={position}>{renderInline(line, `q${index}-${position}`)}</p>
          ))}
        </blockquote>
      );
    case "list": {
      const Tag = block.ordered ? "ol" : "ul";
      return (
        <Tag className="md-list" key={index}>
          {block.items.map((item, position) => (
            <li key={position} data-depth={item.depth || undefined}>
              {renderInline(item.text, `l${index}-${position}`)}
            </li>
          ))}
        </Tag>
      );
    }
    case "table":
      return (
        <div className="md-table-scroll" key={index}>
          <table className="md-table">
            <thead>
              <tr>{block.header.map((cell, position) => <th key={position}>{renderInline(cell, `th${index}-${position}`)}</th>)}</tr>
            </thead>
            <tbody>
              {block.rows.map((row, rowIndex) => (
                <tr key={rowIndex}>
                  {row.map((cell, position) => <td key={position}>{renderInline(cell, `td${index}-${rowIndex}-${position}`)}</td>)}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    case "rule":
      return <hr className="md-rule" key={index} />;
  }
}

/**
 * `memo` importa aqui mais do que em qualquer outro componente da tela: numa
 * thread longa, toda mensagem ja fechada e imutavel, e re-parsear o Markdown de
 * todas elas a cada token da resposta em curso era o custo que a auditoria
 * apontou como critico.
 */
export const Markdown = memo(function Markdown({ source }: { source: string }) {
  return <div className="md">{parseBlocks(source).map(renderBlock)}</div>;
});
