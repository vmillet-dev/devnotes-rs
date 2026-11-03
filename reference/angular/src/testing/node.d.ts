// The few Node built-ins the golden specs use: the project carries no `@types/node`.
declare module 'node:fs' {
  export function mkdirSync(path: string, options?: { recursive?: boolean }): void;
  export function readFileSync(path: string, encoding: 'utf8'): string;
  export function readdirSync(path: string): string[];
  export function writeFileSync(path: string, content: string | Uint8Array): void;
}

declare module 'node:path' {
  export function join(...parts: string[]): string;
}
