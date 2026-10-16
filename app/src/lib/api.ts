import { type CmdError, commands } from "./bindings";
import { errorText } from "./i18n";

/** Error thrown by every command wrapper; carries the backend's kind. */
export class ApiError extends Error {
  kind: string;
  detail: string;
  constructor(error: CmdError) {
    super(errorText(error.kind));
    this.name = "ApiError";
    this.kind = error.kind;
    this.detail = error.message;
  }
}

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CmdError };

type Commands = typeof commands;
type Unwrapped = {
  [K in keyof Commands]: Commands[K] extends (...args: infer A) => Promise<Result<infer T>>
    ? (...args: A) => Promise<T>
    : Commands[K];
};

function isResult(value: unknown): value is Result<unknown> {
  return (
    !!value && typeof value === "object" && "status" in value && (value.status === "ok" || value.status === "error")
  );
}

/** The typed commands, throwing [`ApiError`] instead of returning results. */
export const api = new Proxy({} as Unwrapped, {
  get(_target, name: string) {
    const fn = (commands as Record<string, (...args: unknown[]) => Promise<unknown>>)[name];
    return async (...args: unknown[]) => {
      const value = await fn(...args);
      if (!isResult(value)) return value;
      if (value.status === "error") throw new ApiError(value.error);
      return value.data;
    };
  },
});

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return errorText("other");
}
