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
  moteursStatut,
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

/** L'etat de chaque moteur externe, une fois interroge. */
let moteurs: EngineStatus[] = [];
/** L'identifiant du moteur en cours de telechargement, le cas echeant. */
let installationEnCours: string | null = null;
/** Permet de redessiner depuis le suivi d'installation. */
let redessinerEcran: (() => void) | null = null;

/** Les formats pour lesquels Pandoc change reellement le resultat. */
const CIBLES_PANDOC = new Set(["docx", "markdown", "html", "text"]);

/** Une taille en octets, arrondie au mega-octet. */
function enMo(octets: number): string {
  return `${Math.round(octets / (1024 * 1024))} Mo`;
}

/** Le moteur pertinent pour le format de sortie choisi. */
function moteurPourLaCible(): { id: string; argument: string } | null {
  // Vers un PDF, c'est la fidelite de mise en page qui fait la difference :
  // polices, images, tableaux. Le moteur haute fidelite est seul a la rendre.
  if (cible === "pdf") {
    return {
      id: "libreoffice",
      argument:
        "Le moteur intégré produit un PDF de texte brut, sans styles ni images. " +
        "Le moteur haute fidélité conserve la mise en page d'origine, et ouvre " +
        "en plus les .doc, .odt, .xlsx et .pptx.",
    };
  }
  // Vers un format balisé, Pandoc est meilleur et bien plus rapide.
  if (CIBLES_PANDOC.has(cible)) {
    return {
      id: "pandoc",
      argument:
        "Le moteur intégré rend la structure du document mais perd les tableaux " +
        "et les notes de bas de page. Le moteur avancé les conserve.",
    };
  }
  return null;
}

/**
 * La carte du moteur pertinent pour la conversion en cours.
 *
 * Elle ne s'affiche que la ou le moteur change reellement le resultat :
 * proposer un telechargement de plusieurs centaines de megaoctets pour une
 * conversion que l'outil assure deja tres bien serait deplace.
 */
function carteMoteur(): string {
  const pertinent = moteurPourLaCible();
  if (!pertinent) return "";

  const moteur = moteurs.find((candidat) => candidat.id === pertinent.id);
  if (!moteur) return "";

  if (installationEnCours === moteur.id) {
    return `<div class="engine-card" data-state="installing">
      <div class="engine-card__head">
        <span class="engine-card__dot"></span>
        <span class="engine-card__name">Téléchargement en cours</span>
        <span class="engine-card__badge" id="moteur-pourcent">0 %</span>
      </div>
      <div class="progress"><div class="progress__bar" id="moteur-barre"></div></div>
      <p class="engine-card__note">Vous pouvez continuer à utiliser l'outil pendant ce temps.</p>
    </div>`;
  }

  if (moteur.installed) {
    return `<div class="engine-card" data-state="ready">
      <div class="engine-card__head">
        <span class="engine-card__dot"></span>
        <span class="engine-card__name">${escapeHtml(moteur.label)} actif</span>
        <span class="engine-card__badge">${escapeHtml(moteur.version ?? "")}</span>
      </div>
      <p class="engine-card__note">Vos conversions utilisent la meilleure qualité disponible.</p>
    </div>`;
  }

  return `<div class="engine-card" data-state="missing">
    <div class="engine-card__head">
      <span class="engine-card__dot"></span>
      <span class="engine-card__name">Meilleur résultat disponible</span>
      <span class="engine-card__badge">${enMo(moteur.downloadBytes)} à télécharger</span>
    </div>
    <p class="engine-card__note">
      ${escapeHtml(pertinent.argument)}
      Compter ${enMo(moteur.installedBytes)} sur le disque.
    </p>
    <button class="btn btn--secondary btn--sm" type="button" id="moteur-installer"
            data-moteur-id="${escapeHtml(moteur.id)}">
      Activer le meilleur résultat
    </button>
  </div>`;
}

/** Interroge le moteur et rafraichit l'ecran quand la reponse arrive. */
async function rafraichirMoteur(): Promise<void> {
  try {
    moteurs = await moteursStatut();
  } catch {
    // Hors de la fenetre Tauri, la carte reste simplement absente.
    moteurs = [];
  }
  redessinerEcran?.();
}

/**
 * Telecharge le moteur en suivant sa progression.
 *
 * La barre est mise a jour directement, sans redessiner l'ecran : un rendu
 * complet a chaque paquet recu ferait clignoter toute la colonne.
 */
async function lancerInstallation(id: string, redessiner: () => void): Promise<void> {
  installationEnCours = id;
  redessiner();

  const cesser = await suivreInstallation((progression) => {
    if (progression.id !== id) return;
    const { recus, attendus } = progression;
    const fraction = attendus > 0 ? Math.min(recus / attendus, 1) : 0;
    document
      .querySelector<HTMLElement>("#moteur-barre")
      ?.style.setProperty("--progress", `${(fraction * 100).toFixed(1)}%`);
    const pourcent = document.querySelector<HTMLElement>("#moteur-pourcent");
    if (pourcent) pourcent.textContent = `${Math.round(fraction * 100)} %`;
  });

  try {
    await installerMoteur(id);
  } catch (erreur: unknown) {
    const message = erreur instanceof Error ? erreur.message : String(erreur);
    console.error("[Derovia] installation du moteur impossible :", message);
  } finally {
    cesser();
    installationEnCours = null;
    moteurs = await moteursStatut().catch(() => moteurs);
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

      const bouton = root.querySelector<HTMLButtonElement>("#moteur-installer");
      bouton?.addEventListener("click", () => {
        const id = bouton.dataset["moteurId"];
        if (id) void lancerInstallation(id, redessiner);
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
