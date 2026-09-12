/// <reference types="vite/client" />

/**
 * Les variables de configuration lues au moment de la compilation.
 *
 * Elles viennent de `.env`, qui n'est pas versionne. Sans cette declaration,
 * `import.meta.env.VITE_SUPABASE_URL` serait de type `any` et une faute de
 * frappe passerait inapercue jusqu'a l'execution.
 */
interface ImportMetaEnv {
  readonly VITE_SUPABASE_URL?: string;
  readonly VITE_SUPABASE_ANON_KEY?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
