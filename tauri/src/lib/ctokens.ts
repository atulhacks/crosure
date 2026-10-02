/** A highlighted piece of a pseudo-C line. */
export interface CToken {
  text: string;
  kind: "keyword" | "type" | "number" | "string" | "comment" | "call" | "ident" | "plain";
}

const KEYWORDS = new Set(
  "if else while for do return switch case default break continue goto sizeof".split(" "),
);

const TYPES =
  /^(void|char|short|int|long|float|double|bool|unsigned|signed|const|struct|union|enum|code|byte|word|dword|qword|undefined\d*|u?int\d+_t|size_t|uchar|ushort|uint|ulong|longlong|ulonglong)$/;

const TOKEN =
  /(\/\/.*$)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')|(0x[0-9a-fA-F]+|\d+)|([A-Za-z_][\w.]*)|(\s+|.)/g;

/**
 * Splits one line of pseudo-C into highlight tokens. An identifier followed
 * by `(` is a call.
 *
 * @example tokenizeC("return f(1);").map((t) => t.kind)
 * // ["keyword", "plain", "call", "plain", "number", "plain"]
 */
export function tokenizeC(line: string): CToken[] {
  const out: CToken[] = [];
  for (const m of line.matchAll(TOKEN)) {
    const [text, comment, str, num, ident] = m;
    let kind: CToken["kind"] = "plain";
    if (comment) kind = "comment";
    else if (str) kind = "string";
    else if (num) kind = "number";
    else if (ident) {
      const after = line.slice((m.index ?? 0) + text.length).trimStart();
      if (KEYWORDS.has(ident)) kind = "keyword";
      else if (TYPES.test(ident)) kind = "type";
      else kind = after.startsWith("(") ? "call" : "ident";
    }
    const last = out.at(-1);
    if (kind === "plain" && last?.kind === "plain") last.text += text;
    else out.push({ text, kind });
  }
  return out;
}
