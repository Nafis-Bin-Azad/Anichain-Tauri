declare namespace NodeJS {
  interface ProcessEnv {
    NEXT_PUBLIC_TAURI_BINDINGS_WINDOW_LABEL?: string;
    NEXT_PUBLIC_TAURI_PLATFORM?: string;
    NEXT_PUBLIC_TAURI_FAMILY?: string;
    NEXT_PUBLIC_TAURI_PLATFORM_VERSION?: string;
    NEXT_PUBLIC_TAURI_PLATFORM_TYPE?: string;
    NEXT_PUBLIC_TAURI_ARCH?: string;
    NEXT_PUBLIC_TAURI_DEBUG?: string;
  }
}

interface Window {
  __TAURI__?: {
    invoke(cmd: string, args?: Record<string, unknown>): Promise<any>;
  };
}

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
