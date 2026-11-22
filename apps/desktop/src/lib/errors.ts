// Errors are shown by their stable codes: the code maps to an i18n key (`errors.<code>`);
// an unknown code falls back to the English text from Rust.
import type { i18n as I18n } from "i18next";
import type { AppError } from "@/bindings";

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    "code" in e &&
    "message" in e &&
    typeof (e as AppError).code === "string"
  );
}

/** A translated summary of any thrown value, plus the technical detail when known. */
export function describeError(i18n: I18n, e: unknown): { summary: string; detail?: string } {
  if (isAppError(e)) {
    const key = `errors.${e.code}`;
    if (i18n.exists(key)) {
      return { summary: i18n.t(key as "errors.generic"), detail: e.message };
    }
    return { summary: e.message };
  }
  const text = e instanceof Error ? e.message : String(e);
  return { summary: i18n.t("errors.generic"), detail: text };
}
