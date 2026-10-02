/** One row of a hex dump. */
export interface HexRow {
  addr: number;
  hex: string;
  ascii: string;
}

/**
 * Splits a hex string into 16-byte rows with an ASCII column.
 * @example hexRows(0x10, "41420a")[0] // { addr: 16, hex: "41 42 0a", ascii: "AB." }
 */
export function hexRows(base: number, hex: string, width = 16): HexRow[] {
  const bytes = hex.match(/../g) ?? [];
  const rows: HexRow[] = [];
  for (let i = 0; i < bytes.length; i += width) {
    const chunk = bytes.slice(i, i + width);
    rows.push({
      addr: base + i,
      hex: chunk.join(" "),
      ascii: chunk
        .map((b) => {
          const c = parseInt(b, 16);
          return c >= 0x20 && c < 0x7f ? String.fromCharCode(c) : ".";
        })
        .join(""),
    });
  }
  return rows;
}
