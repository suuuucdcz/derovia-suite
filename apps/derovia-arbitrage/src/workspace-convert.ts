/**
 * Derovia Convertisseur — conversion de documents et d'images.
 *
 * Tout le comportement commun aux outils de fichiers — file d'attente, zone de
 * depot, enregistrement, etats — vit dans [`mountFileTool`]. Ce module ne decrit
 * que ce qui est propre au convertisseur.
 */

import {
  convertirDocument,
  installerMoteur,
  moteurStatut,
  suivreInstallation,
} from "./api";
import { escapeHtml } from "./dom";
import { mountFileTool } from "./file-tool";
import { formatBytes } from "./format";
import type { ConversionResult, EngineStatus } from "./types";

/** Les formats de sortie proposes, dans l'ordre du menu. */
const CIBLES: ReadonlyArray<{ value: string; label: string }> = [
  { value: "pdf", label: "Document PDF (.pdf)" },
  { value: "docx", label: "Microsoft Word (.docx)" },
  { value: "markdown", label: "Document Markdown (.md)" },
  { value: "html", label: "Page web (.html)" },
  { value: "text", label: "Texte brut (.txt)" },
  { value: "png", label: "Image PNG (.png)" },
  { value: "jpeg", label: "Image JPEG (.jpg)" },
  { value: "webp", label: "Image WebP (.webp)" },
];

const ICONE = `
<svg class="dropzone__icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor"
     stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
  <polyline points="14 2 14 8 20 8" />
  <line x1="12" y1="18" x2="12" y2="12" />
  <polyline points="9 15 12 12 15 15" />
</svg>`;

/** Le format cible, conserve d'un rendu a l'autre. */
let cible = "pdf";

/** L'etat du moteur Pandoc, une fois interroge. */
let moteur: EngineStatus | null = null;
/** Vrai pendant le telechargement. */
let installation = false;
/** Permet de redessiner depuis le suivi d'installation. */
let redessinerEcran: (() => void) | null = null;

/** Les formats pour lesquels Pandoc change reellement le resultat. */
const CIBLES_PANDOC = new Set(["docx", "markdown", "html", "text"]);

/** Une taille en octets, arrondie au mega-octet. */
function enMo(octets: number): string {
  return `${Math.round(octets / (1024 * 1024))} Mo`;
}

/**
 * La carte d'etat du moteur avance.
 *
 * Elle n'apparait que pour les formats ou Pandoc apporte quelque chose : la
 * proposer avant une conversion vers PDF, qu'il ne sait pas produire, serait
 * trompeur.
 */
function carteMoteur(): string {
  if (!moteur || !CIBLES_PANDOC.has(cible)) return "";

  if (installation) {
    return `<div class="engine-card" data-state="installing">
      <div class="engine-card__head">
        <span class="engine-card__dot"></span>
        <span class="engine-card__name">Installation du moteur avancé</span>
        <span class="engine-card__badge" id="moteur-pourcent">0 %</span>
      </div>
      <div class="progress"><div class="progress__bar" id="moteur-barre"></div></div>
      <p class="engine-card__note">Téléchargement en cours. Vous pouvez continuer à utiliser l'outil.</p>
    </div>`;
  }

  if (moteur.installed) {
    return `<div class="engine-card" data-state="ready">
      <div class="engine-card__head">
        <span class="engine-card__dot"></span>
        <span class="engine-card__name">Moteur avancé actif</span>
        <span class="engine-card__badge">${escapeHtml(moteur.version ?? "Pandoc")}</span>
      </div>
      <p class="engine-card__note">Tableaux, notes de bas de page et styles sont conservés.</p>
    </div>`;
  }

  return `<div class="engine-card" data-state="missing">
    <div class="engine-card__head">
      <span class="engine-card__dot"></span>
      <span class="engine-card__name">Moteur avancé disponible</span>
      <span class="engine-card__badge">Pandoc</span>
    </div>
    <p class="engine-card__note">
      Le moteur intégré rend la structure du document mais perd les tableaux et
      les notes. Pandoc les conserve. ${enMo(moteur.downloadBytes)} à télécharger,
      ${enMo(moteur.installedBytes)} sur le disque.
    </p>
    <button class="btn btn--secondary btn--sm" type="button" id="moteur-installer">
      Installer le moteur avancé
    </button>
  </div>`;
}

/** Interroge le moteur et rafraichit l'ecran quand la reponse arrive. */
async function rafraichirMoteur(): Promise<void> {
  try {
    moteur = await moteurStatut();
  } catch {
    // Hors de la fenetre Tauri, la carte reste simplement absente.
    moteur = null;
  }
  redessinerEcran?.();
}

/**
 * Telecharge le moteur en suivant sa progression.
 *
 * La barre est mise a jour directement, sans redessiner l'ecran : un rendu
 * complet a chaque paquet recu ferait clignoter toute la colonne.
 */
async function lancerInstallation(redessiner: () => void): Promise<void> {
  installation = true;
  redessiner();

  const cesser = await suivreInstallation((recus, attendus) => {
    const fraction = attendus > 0 ? Math.min(recus / attendus, 1) : 0;
    document
      .querySelector<HTMLElement>("#moteur-barre")
      ?.style.setProperty("--progress", `${(fraction * 100).toFixed(1)}%`);
    const pourcent = document.querySelector<HTMLElement>("#moteur-pourcent");
    if (pourcent) pourcent.textContent = `${Math.round(fraction * 100)} %`;
  });

  try {
    moteur = await installerMoteur();
  } catch (erreur: unknown) {
    moteur = await moteurStatut().catch(() => null);
    const message = erreur instanceof Error ? erreur.message : String(erreur);
    console.error("[Derovia] installation du moteur impossible :", message);
  } finally {
    cesser();
    installation = false;
    redessiner();
  }
}

/** Demarre l'espace de travail Convertisseur. */
export function startConvertWorkspace(): void {
  mountFileTool<ConversionResult>({
    id: "convert",
    title: "Convertisseur",
    subtitle: "Word, PDF, images, Markdown, HTML : convertir de l'un vers l'autre",
    icon: ICONE,
    dropHint: "Word (.docx), PDF, images, Markdown, HTML, texte",
    formats: ["DOCX", "PDF", "PNG / JPG", "MD", "HTML", "TXT"],
    emptyTitle: "Aucun fichier en attente",
    emptyLead:
      "Déposez un ou plusieurs documents, à gauche ou directement ici, puis choisissez le format de sortie.",
    emptyCards: [
      { title: "Word ⇄ PDF", note: "OpenXML et PDF vectoriel" },
      { title: "Markdown ⇄ HTML", note: "Structure préservée" },
      { title: "Images → PDF", note: "Mise en page automatique" },
    ],

    optionsHtml: () => `
      ${carteMoteur()}
      <label class="field">
        <span class="field__label">Format de sortie</span>
        <span class="input">
          <select id="convert-target">
            ${CIBLES.map(
              (format) =>
                `<option value="${format.value}"${format.value === cible ? " selected" : ""}>${format.label}</option>`,
            ).join("")}
          </select>
        </span>
      </label>`,

    bindOptions: (root, redessiner) => {
      redessinerEcran = redessiner;

      const select = root.querySelector<HTMLSelectElement>("#convert-target");
      select?.addEventListener("change", () => {
        cible = select.value;
        redessiner();
      });

      root.querySelector<HTMLButtonElement>("#moteur-installer")?.addEventListener("click", () => {
        void lancerInstallation(redessiner);
      });
    },

    actionLabel: (pending) => `Convertir ${pending} fichier${pending > 1 ? "s" : ""}`,
    busyLabel: "Conversion en cours…",
    doneLabel: "Tous les fichiers sont convertis",
    summaryEyebrow: "Conversion terminée",
    summaryNote: "Les fichiers produits sont prêts à être enregistrés.",

    process: (file, bytes) => convertirDocument(file.name, bytes, cible),

    describe: (result) => ({
      badge: result.outputFormat,
      detail: `${formatBytes(result.originalSize)} → ${formatBytes(result.outputSize)}`,
    }),

    pendingDetail: (file) =>
      `${formatBytes(file.size)} • vers ${CIBLES.find((format) => format.value === cible)?.label ?? cible}`,
  });

  void rafraichirMoteur();
}
