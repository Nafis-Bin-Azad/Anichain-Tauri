const isTauriAvailable = () => {
  try {
    return typeof window !== "undefined" && window.__TAURI__ !== undefined;
  } catch {
    return false;
  }
};

export async function invokeTauri<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (!isTauriAvailable()) {
    console.warn("Tauri API not available");
    return Promise.resolve({} as T);
  }
  // @ts-ignore
  return window.__TAURI__.invoke(command, args);
}

export const tauri = {
  invoke: invokeTauri,
};
