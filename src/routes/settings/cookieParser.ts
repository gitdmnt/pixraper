export interface ParsedCookie {
  key: string;
  value: string;
  important: boolean;
}

export const IMPORTANT_KEYS: ReadonlySet<string> = new Set([
  "PHPSESSID",
  "device_token",
]);

/**
 * Cookie 文字列（"key=value; key2=value2" 形式）をパースして ParsedCookie[] を返す。
 * - 空文字列・空白のみは空配列を返す
 * - key または value が空の要素は除外する
 */
export const parseCookies = (raw: string): ParsedCookie[] => {
  if (!raw.trim()) return [];

  return raw
    .split(";")
    .map((pair) => pair.trim())
    .filter((pair) => pair.includes("="))
    .map((pair) => {
      const idx = pair.indexOf("=");
      const key = pair.slice(0, idx).trim();
      const value = pair.slice(idx + 1).trim();
      return { key, value };
    })
    .filter(({ key, value }) => key.length > 0 && value.length > 0)
    .map(({ key, value }) => ({
      key,
      value,
      important: IMPORTANT_KEYS.has(key),
    }));
};

export const hasPhpSessionId = (parsed: ParsedCookie[]): boolean =>
  parsed.some((c) => c.key === "PHPSESSID");
