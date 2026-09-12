/**
 * Derovia Compresseur — reduction du poids des fichiers.
 *
 * Comme le Convertisseur, il s'appuie sur [`mountFileTool`] pour tout ce qui est
 * commun. Ce module ne decrit que ses options et son resume de resultat.
 */

import { compresserFichier } from "./api";
import { mountFileTool } from "./file-tool";
import { formatBytes } from "./format";
import type { CompressResult, CompressionLevel } from "./types";

const ICONE = `
<svg class="dropzone__icon-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor"
     stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d="M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242" />
  <path d="M12 12v9" />
  <path d="m8 17 4 4 4-4" />
</svg>`;

/** Options conservees d'un rendu a l'autre. */
let niveau: CompressionLevel = "balanced";
let qualite = 80;
let redimension = 100;

/** Demarre l'espace de travail Compresseur. */
export function startCompressWorkspace(): void {
  mountFileTool<CompressResult>({
    id: "compress",
    title: "Compresseur",
    subtitle: "Réduire le poids d'un fichier sans le rendre inutilisable",
    icon: ICONE,
    dropHint: "Images (PNG, JPG, WebP, BMP), PDF, documents, archives",
    formats: ["PNG", "JPG", "WEBP", "PDF", "ZIP"],
    emptyTitle: "Aucun fichier en attente",
    emptyLead:
      "Déposez vos fichiers, à gauche ou directement ici. Le fichier rendu n'est jamais plus lourd que l'original.",
    emptyCards: [
      { title: "Images", note: "Réencodage et redimensionnement" },
      { title: "PDF", note: "Flux recompressés" },
      { title: "Tout le reste", note: "Archive ZIP" },
    ],

    optionsHtml: () => `
      <label class="field">
        <span class="field__label">Niveau d'optimisation</span>
        <span class="input">
          <select id="compress-level">
            <option value="balanced"${niveau === "balanced" ? " selected" : ""}>Équilibré — compromis poids / qualité</option>
            <option value="lossless"${niveau === "lossless" ? " selected" : ""}>Sans perte — qualité intégrale</option>
            <option value="maximum"${niveau === "maximum" ? " selected" : ""}>Maximum — poids le plus faible</option>
          </select>
        </span>
      </label>

      <label class="field">
        <span class="field__label">
          Qualité d'image
          <span class="field__value" id="compress-quality-readout">${qualite} %</span>
        </span>
        <input class="slider" type="range" id="compress-quality" min="10" max="100" value="${qualite}" />
      </label>

      <label class="field">
        <span class="field__label">Redimensionner les images</span>
        <span class="input">
          <select id="compress-resize">
            <option value="100"${redimension === 100 ? " selected" : ""}>Dimensions d'origine</option>
            <option value="75"${redimension === 75 ? " selected" : ""}>Réduire à 75 %</option>
            <option value="50"${redimension === 50 ? " selected" : ""}>Réduire à 50 % — pour le web ou l'e-mail</option>
          </select>
        </span>
      </label>`,

    bindOptions: (root, redessiner) => {
      const level = root.querySelector<HTMLSelectElement>("#compress-level");
      level?.addEventListener("change", () => {
        niveau = level.value as CompressionLevel;
        redessiner();
      });

      const slider = root.querySelector<HTMLInputElement>("#compress-quality");
      const readout = root.querySelector<HTMLElement>("#compress-quality-readout");
      // La valeur suit le curseur sans redessiner : un rendu complet a chaque
      // pixel de deplacement ferait perdre la prise sur le curseur.
      slider?.addEventListener("input", () => {
        qualite = Number(slider.value);
        if (readout) readout.textContent = `${qualite} %`;
      });

      const resize = root.querySelector<HTMLSelectElement>("#compress-resize");
      resize?.addEventListener("change", () => {
        redimension = Number(resize.value);
        redessiner();
      });
    },

    actionLabel: (pending) => `Compresser ${pending} fichier${pending > 1 ? "s" : ""}`,
    busyLabel: "Compression en cours…",
    doneLabel: "Tous les fichiers sont traités",
    summaryEyebrow: "Compression terminée",
    summaryNote: "Les fichiers produits sont prêts à être enregistrés.",

    process: (file, bytes) =>
      compresserFichier(file.name, bytes, {
        level: niveau,
        quality: qualite,
        ...(redimension < 100 ? { resizePercent: redimension } : {}),
      }),

    describe: (result) => {
      const [, extension] = /\.([^.]+)$/.exec(result.fileName) ?? [];
      const gain =
        result.reductionPercent > 0
          ? `−${result.reductionPercent.toFixed(1).replace(".", ",")} %`
          : "déjà optimisé";
      return {
        badge: (extension ?? "fichier").toUpperCase(),
        detail: `${formatBytes(result.originalSize)} → ${formatBytes(result.compressedSize)} • ${gain}`,
      };
    },

    pendingDetail: (file) => `${formatBytes(file.size)} • en attente`,
  });
}
