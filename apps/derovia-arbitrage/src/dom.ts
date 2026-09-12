/**
 * Aides communes a tous les espaces de travail.
 *
 * Deux choses y vivent parce qu'elles etaient recopiees dans chaque outil, avec
 * des variantes : l'echappement du texte insere dans du HTML, et la reprise en
 * main de la barre de titre.
 */

/**
 * Rend un texte sur a inserer dans du HTML.
 *
 * Les noms de fichiers viennent de l'utilisateur. Sans echappement, un fichier
 * nomme `R&D <2024>.docx` casse l'affichage, et un nom construit pour nuire y
 * injecterait du balisage.
 */
export function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** Les elements de la barre de titre qu'un outil reprend a son ouverture. */
export interface WorkspaceChrome {
  /** Le nom de l'outil. */
  title: string;
  /** La phrase qui le situe, sous le titre. */
  subtitle: string;
  /** Le balisage des boutons d'action, vide si l'outil n'en a pas. */
  actions?: string;
}

/**
 * Installe la barre de titre de l'outil qui s'ouvre.
 *
 * **Chaque outil doit l'appeler a chaque ouverture**, pas seulement la
 * premiere. Un outil qui se contentait de vider `.topbar__actions` laissait les
 * boutons du precedent detruits : en revenant dessus, l'utilisateur retrouvait
 * le mauvais titre et une barre vide.
 */
export function setWorkspaceChrome(chrome: WorkspaceChrome): void {
  const title = document.querySelector<HTMLElement>(".topbar__title h1");
  const subtitle = document.querySelector<HTMLElement>("#app-subtitle");
  const actions = document.querySelector<HTMLElement>(".topbar__actions");

  if (title) title.textContent = chrome.title;
  if (subtitle) subtitle.textContent = chrome.subtitle;
  if (actions) actions.innerHTML = chrome.actions ?? "";
}

/**
 * Empeche le navigateur d'ouvrir un fichier depose a cote d'une zone de depot.
 *
 * Sans cela, laisser tomber un document hors de la zone prevue remplace la
 * fenetre de l'application par le fichier lui-meme.
 */
let dragDropNeutralised = false;

export function neutraliserDepotGlobal(): void {
  if (dragDropNeutralised) return;
  dragDropNeutralised = true;
  for (const type of ["dragover", "drop"] as const) {
    window.addEventListener(type, (event) => event.preventDefault());
  }
}
