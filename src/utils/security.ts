export const DANGEROUS_TOKENS = [
  "--exec",
  "--exec-before-download",
  "--exec-after-download",
  "--config-location",
  "--load-info-json",
  "--external-downloader",
  "--external-downloader-args",
  "--alias",
  ";",
  "&&",
  "||",
  "|",
  "`",
  "$(",
] as const;

export interface ValidationResult {
  valid: boolean;
  blockedToken?: string;
}

export function validateSafeArguments(text: string | null | undefined): ValidationResult {
  if (!text) return { valid: true };
  const lower = text.toLowerCase();
  for (const token of DANGEROUS_TOKENS) {
    if (lower.includes(token.toLowerCase())) {
      return { valid: false, blockedToken: token };
    }
  }
  return { valid: true };
}
