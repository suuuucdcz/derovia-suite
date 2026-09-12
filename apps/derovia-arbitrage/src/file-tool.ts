/**
 * Le squelette commun aux outils qui traitent des fichiers.
 *
 * Le Convertisseur et le Compresseur font la meme chose a 90 % : accumuler des
 * fichiers deposes, les traiter un par un, presenter le resultat et permettre
 * de l'enregistrer. Seuls changent les options, le traitement lui-meme et la
 * facon de resumer un resultat.
 *
 * Ces deux ecrans existaient en double, a plus de cinq cents lignes chacun,
 * avec leurs divergences : l'un affichait les erreurs d'une facon, l'autre
 * d'une autre, et chacun recopiait ses couleurs en dur. Tout ce qui est commun
 * vit desormais ici, et chaque outil se reduit a sa configuration.
 */

import { messageDErreur, inTauri, lireFichier, ouvrirDansExplorateur, sauvegarderFichier } from "./api";
import { escapeHtml, neutraliserDepotGlobal, setWorkspaceChrome } from "./dom";
import { formatBytes } from "./format";

/** Ce que tout resultat d'outil de fichier doit exposer. */
export interface FileToolResult {
  /** Le nom du fichier produit. */
  fileName: string;
  /** Ses octets. */
  data: number[];
}

/** Une carte d'exemple affichee tant qu'aucun fichier n'a ete depose. */
export interface FileToolCard {
  /** Le titre de la carte. */
  title: string;
  /** La precision, en une poignee de mots. */
  note: string;
}

/** Le resume d'un fichier traite, tel qu'il apparait dans la liste. */
export interface ResultSummary {
  /** L'etiquette de format, en capitales. */
  badge: string;
  /** La ligne de detail sous le nom du fichier. */
  detail: string;
}

/** Tout ce qui distingue un outil de fichiers d'un autre. */
export interface FileToolConfig<R extends FileToolResult> {
  /** Identifiant court, utilise pour les identifiants DOM. */
  id: string;
  /** Le nom de l'outil, en barre de titre. */
  title: string;
  /** La phrase qui le situe. */
  subtitle: string;
  /** L'icone de la zone de depot. */
  icon: string;
  /** Ce que la zone de depot accepte, en clair. */
  dropHint: string;
  /** Les formats mis en avant sous la zone de depot. */
  formats: string[];
  /** Le titre de l'ecran vide. */
  emptyTitle: string;
  /** Le texte de l'ecran vide. */
  emptyLead: string;
  /** Trois exemples de ce que l'outil sait faire. */
  emptyCards: FileToolCard[];
  /** Le contenu de la carte d'options, hors en-tete. */
  optionsHtml: () => string;
  /** Branche les controles d'options ; `redessiner` rafraichit l'ecran. */
  bindOptions: (root: HTMLElement, redessiner: () => void) => void;
  /** Le libelle du bouton d'action selon le nombre de fichiers en attente. */
  actionLabel: (pending: number) => string;
  /** Le libelle pendant le traitement. */
  busyLabel: string;
  /** Le libelle quand tout est traite. */
  doneLabel: string;
  /** Le sur-titre du bandeau de synthese. */
  summaryEyebrow: string;
  /** La phrase du bandeau de synthese. */
  summaryNote: string;
  /** Le traitement d'un fichier. */
  process: (file: File, bytes: number[]) => Promise<R>;
  /** Le resume d'un resultat. */
  describe: (result: R) => ResultSummary;
  /** La ligne de detail d'un fichier encore en attente. */
  pendingDetail: (file: File) => string;
}

interface QueueItem<R> {
  id: string;
  file: File;
  result?: R | undefined;
  error?: string | undefined;
  loading?: boolean | undefined;
  savedPath?: string | undefined;
}

/**
 * Associe une extension a une famille de couleur du design system.
 *
 * Les teintes elles-memes vivent dans les jetons : ecrites ici, elles
 * ignoreraient le theme sombre.
 */
function badgeKind(extension: string): string {
  switch (extension.toLowerCase()) {
    case "pdf":
      return "pdf";
    case "png":
    case "jpg":
    case "jpeg":
    case "webp":
    case "bmp":
      return "image";
    case "doc":
    case "docx":
    case "txt":
      return "doc";
    case "md":
    case "markdown":
    case "html":
    case "htm":
      return "markup";
    case "zip":
    case "rar":
    case "7z":
    case "gz":
    case "tar":
      return "archive";
    default:
      return "neutral";
  }
}

/** L'extension d'un nom de fichier, en capitales. */
function extensionOf(fileName: string): string {
  const [, ext] = /\.([^.]+)$/.exec(fileName) ?? [];
  return (ext ?? "fichier").toUpperCase();
}

/** Declenche un telechargement navigateur, en dernier recours. */
function telechargerDepuisLeNavigateur(fileName: string, data: number[]): void {
  const url = URL.createObjectURL(new Blob([new Uint8Array(data)]));
  const lien = document.createElement("a");
  lien.href = url;
  lien.download = fileName;
  document.body.append(lien);
  lien.click();
  lien.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 2000);
}

/**
 * Monte un outil de fichiers dans la coquille applicative.
 *
 * Appelable a chaque ouverture de l'outil : l'etat repart propre et la barre de
 * titre est reinstallee.
 */
export function mountFileTool<R extends FileToolResult>(config: FileToolConfig<R>): void {
  neutraliserDepotGlobal();
  setWorkspaceChrome({ title: config.title, subtitle: config.subtitle });

  const formColumn = document.querySelector<HTMLElement>("#form-column");
  const resultColumn = document.querySelector<HTMLElement>("#result-column");
  if (!formColumn || !resultColumn) return;

  let queue: QueueItem<R>[] = [];

  const ajouter = (files: FileList | File[]): void => {
    for (const file of Array.from(files)) {
      queue.push({ id: `${config.id}-${crypto.randomUUID()}`, file });
    }
    render();
  };

  const enregistrer = async (item: QueueItem<R>): Promise<void> => {
    if (!item.result) return;
    const { fileName, data } = item.result;
    if (!inTauri()) {
      telechargerDepuisLeNavigateur(fileName, data);
      render();
      return;
    }
    try {
      const saved = await sauvegarderFichier(fileName, data);
      item.savedPath = saved.chemin;
      item.error = undefined;
      void ouvrirDansExplorateur(saved.chemin);
    } catch (erreur: unknown) {
      // L'echec est affiche : le laisser passer sous silence donnait un bouton
      // qui ne faisait visiblement rien.
      item.error = messageDErreur(erreur);
    }
    render();
  };

  const traiter = async (): Promise<void> => {
    for (const item of queue) {
      if (item.result || item.loading) continue;
      item.loading = true;
      item.error = undefined;
      render();
      try {
        const bytes = await lireFichier(item.file);
        item.result = await config.process(item.file, bytes);
      } catch (erreur: unknown) {
        item.error = messageDErreur(erreur);
      }
      item.loading = false;
      render();
    }
  };

  function itemMarkup(item: QueueItem<R>): string {
    const nom = escapeHtml(item.file.name);

    if (item.loading) {
      return `<div class="file-item">
        <div class="file-item__info">
          <span class="file-item__badge" data-kind="${badgeKind(extensionOf(item.file.name))}">${escapeHtml(extensionOf(item.file.name))}</span>
          <div>
            <div class="file-item__name">${nom}</div>
            <div class="file-item__size">${escapeHtml(formatBytes(item.file.size))} • traitement en cours…</div>
          </div>
        </div>
        <div class="file-item__actions"><span class="pill">En cours…</span></div>
      </div>`;
    }

    if (item.error) {
      return `<div class="file-item" data-state="error">
        <div class="file-item__info">
          <span class="file-item__badge" data-kind="error">!</span>
          <div>
            <div class="file-item__name">${nom}</div>
            <div class="file-item__size" data-tone="negative">${escapeHtml(item.error)}</div>
          </div>
        </div>
        <div class="file-item__actions">
          <span class="pill" data-tone="negative">Échec</span>
          <button class="file-item__remove" type="button" data-remove="${escapeHtml(item.id)}" title="Retirer">✕</button>
        </div>
      </div>`;
    }

    if (item.result) {
      const resume = config.describe(item.result);
      const enregistre = Boolean(item.savedPath);
      return `<div class="file-item">
        <div class="file-item__info">
          <span class="file-item__badge" data-kind="${badgeKind(resume.badge)}">${escapeHtml(resume.badge)}</span>
          <div>
            <div class="file-item__name">${escapeHtml(item.result.fileName)}</div>
            <div class="file-item__size">
              ${escapeHtml(resume.detail)}${enregistre ? ` • <span class="file-item__saved">✓ enregistré</span>` : ""}
            </div>
          </div>
        </div>
        <div class="file-item__actions">
          ${
            enregistre
              ? `<button class="btn btn--secondary btn--sm" type="button" data-open="${escapeHtml(item.savedPath ?? "")}">Ouvrir</button>
                 <button class="btn btn--ghost btn--sm" type="button" data-download="${escapeHtml(item.id)}">Réenregistrer</button>`
              : `<button class="btn btn--secondary btn--sm" type="button" data-download="${escapeHtml(item.id)}">Enregistrer</button>`
          }
          <button class="file-item__remove" type="button" data-remove="${escapeHtml(item.id)}" title="Retirer">✕</button>
        </div>
      </div>`;
    }

    return `<div class="file-item">
      <div class="file-item__info">
        <span class="file-item__badge" data-kind="${badgeKind(extensionOf(item.file.name))}">${escapeHtml(extensionOf(item.file.name))}</span>
        <div>
          <div class="file-item__name">${nom}</div>
          <div class="file-item__size">${escapeHtml(config.pendingDetail(item.file))}</div>
        </div>
      </div>
      <div class="file-item__actions">
        <span class="pill">En attente</span>
        <button class="file-item__remove" type="button" data-remove="${escapeHtml(item.id)}" title="Retirer">✕</button>
      </div>
    </div>`;
  }

  function render(): void {
    const termines = queue.filter((item) => item.result).length;
    const enAttente = queue.filter((item) => !item.result && !item.loading && !item.error).length;
    const occupe = queue.some((item) => item.loading);

    formColumn!.innerHTML = `
      <section class="card rise">
        <div class="card__header">
          <h2 class="card__title">Ajouter des fichiers</h2>
          <span class="card__hint">${queue.length > 0 ? `${queue.length} en liste` : "Glisser-déposer"}</span>
        </div>

        <div class="dropzone" id="${config.id}-dropzone" tabindex="0" role="button"
             aria-label="Choisir des fichiers">
          ${config.icon}
          <div class="dropzone__title">Glissez vos fichiers ici ou cliquez pour parcourir</div>
          <div class="dropzone__hint">${escapeHtml(config.dropHint)}</div>
          <div class="dropzone__formats">
            ${config.formats.map((f) => `<span class="dropzone__format-pill">${escapeHtml(f)}</span>`).join("")}
          </div>
        </div>
        <input type="file" id="${config.id}-input" class="visually-hidden" multiple />

        <div class="option-stack">${config.optionsHtml()}</div>

        <div class="option-stack">
          <button class="btn btn--primary btn--block" type="button" id="${config.id}-run"
                  ${queue.length === 0 || occupe || enAttente === 0 ? "disabled" : ""}>
            ${escapeHtml(
              occupe
                ? config.busyLabel
                : enAttente > 0
                  ? config.actionLabel(enAttente)
                  : queue.length > 0
                    ? config.doneLabel
                    : "Sélectionnez des fichiers",
            )}
          </button>
        </div>
      </section>
    `;

    if (queue.length === 0) {
      resultColumn!.innerHTML = `
        <section class="card rise" id="${config.id}-empty">
          <div class="empty-state">
            <div class="empty-state__icon">${config.icon}</div>
            <h2 class="empty-state__title">${escapeHtml(config.emptyTitle)}</h2>
            <p class="empty-state__lead">${escapeHtml(config.emptyLead)}</p>
            <div class="empty-state__grid">
              ${config.emptyCards
                .map(
                  (card) => `<div class="stat">
                    <span class="stat__label">${escapeHtml(card.title)}</span>
                    <span class="stat__note">${escapeHtml(card.note)}</span>
                  </div>`,
                )
                .join("")}
            </div>
          </div>
        </section>
      `;
    } else {
      const synthese =
        termines > 0
          ? `<section class="summary-bar rise">
              <div>
                <div class="summary-bar__eyebrow">${escapeHtml(config.summaryEyebrow)}</div>
                <div class="summary-bar__headline">${termines} fichier${termines > 1 ? "s" : ""} prêt${termines > 1 ? "s" : ""}</div>
                <div class="summary-bar__note">${escapeHtml(config.summaryNote)}</div>
              </div>
              <button class="btn btn--primary" type="button" id="${config.id}-save-all">
                Tout enregistrer (${termines})
              </button>
            </section>`
          : "";

      resultColumn!.innerHTML = `
        ${synthese}
        <section class="card rise">
          <div class="card__header">
            <h2 class="card__title">Fichiers (${queue.length})</h2>
            <button class="btn btn--ghost btn--sm" type="button" id="${config.id}-clear">Vider la liste</button>
          </div>
          <div class="file-list">${queue.map((item) => itemMarkup(item)).join("")}</div>
        </section>
      `;
    }

    wire();
  }

  function accepterDepot(zone: HTMLElement | null): void {
    if (!zone) return;
    for (const type of ["dragenter", "dragover"] as const) {
      zone.addEventListener(type, (event) => {
        event.preventDefault();
        event.stopPropagation();
        if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
        zone.classList.add("is-dragover");
      });
    }
    zone.addEventListener("dragleave", () => zone.classList.remove("is-dragover"));
    zone.addEventListener("drop", (event) => {
      event.preventDefault();
      event.stopPropagation();
      zone.classList.remove("is-dragover");
      if (event.dataTransfer?.files.length) ajouter(event.dataTransfer.files);
    });
  }

  function wire(): void {
    const dropzone = formColumn!.querySelector<HTMLElement>(`#${config.id}-dropzone`);
    const input = formColumn!.querySelector<HTMLInputElement>(`#${config.id}-input`);

    dropzone?.addEventListener("click", () => input?.click());
    dropzone?.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        input?.click();
      }
    });
    input?.addEventListener("change", () => {
      if (input.files?.length) ajouter(input.files);
      input.value = "";
    });

    accepterDepot(dropzone);
    accepterDepot(resultColumn!.querySelector<HTMLElement>(`#${config.id}-empty`));

    config.bindOptions(formColumn!, render);

    formColumn!
      .querySelector<HTMLButtonElement>(`#${config.id}-run`)
      ?.addEventListener("click", () => void traiter());

    resultColumn!.querySelector<HTMLButtonElement>(`#${config.id}-clear`)?.addEventListener(
      "click",
      () => {
        queue = [];
        render();
      },
    );

    resultColumn!
      .querySelector<HTMLButtonElement>(`#${config.id}-save-all`)
      ?.addEventListener("click", () => {
        void (async () => {
          for (const item of queue) {
            if (item.result) await enregistrer(item);
          }
        })();
      });

    for (const bouton of resultColumn!.querySelectorAll<HTMLElement>("[data-remove]")) {
      bouton.addEventListener("click", () => {
        queue = queue.filter((item) => item.id !== bouton.dataset["remove"]);
        render();
      });
    }

    for (const bouton of resultColumn!.querySelectorAll<HTMLElement>("[data-download]")) {
      bouton.addEventListener("click", () => {
        const item = queue.find((candidate) => candidate.id === bouton.dataset["download"]);
        if (item) void enregistrer(item);
      });
    }

    for (const bouton of resultColumn!.querySelectorAll<HTMLElement>("[data-open]")) {
      bouton.addEventListener("click", () => {
        const chemin = bouton.dataset["open"];
        if (chemin) void ouvrirDansExplorateur(chemin);
      });
    }
  }

  render();
}
