declare module "@tauri-apps/api/tauri" {
  export function invoke<T>(
    cmd: string,
    args?: Record<string, unknown>
  ): Promise<T>;
}

declare module "@tauri-apps/api/dialog" {
  export function open(options?: {
    multiple?: boolean;
    filters?: { name: string; extensions: string[] }[];
  }): Promise<string | string[] | null>;
}

declare module "@tauri-apps/api/path" {
  export function appDir(): Promise<string>;
  export function join(...paths: string[]): Promise<string>;
}
