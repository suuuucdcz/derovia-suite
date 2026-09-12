/**
 * L'ecran de preparation du premier lancement.
 *
 * Derovia s'appuie sur des moteurs externes, trop lourds pour tenir dans
 * l'installeur. Cet ecran les installe, avec une barre de progression.
 *
 * **Il ne prend jamais l'application en otage.** Sur une connexion lente, ces
 * telechargements durent plusieurs minutes : on peut entrer dans la suite
 * pendant qu'ils se poursuivent en arriere-plan, et chaque outil s'active des
 * que son moteur est pret. Un ecran bloquant aurait ete plus simple a ecrire,
 * et insupportable a utiliser.
 *
 * **Rien ne se telecharge sans avoir ete annonce.** Les moteurs optionnels ont
 * leur propre bouton : un telechargement de plusieurs centaines de megaoctets
 * qui demarre parce qu'on a clique sur autre chose trahit ce que le bouton
 * promettait, et fait douter de tout le reste.
 */

import { installerMoteur, moteursStatut, suivreInstallation } from "./api";
import { escapeHtml } from "./dom";
import { formatBytes } from "./format";
import type { EngineStatus } from "./types";

/** Retient qu'on a deja propose l'installation, pour ne pas la reproposer. */
const CLE_VU = "derovia.preparation.vue";

/** L'etat courant de chaque moteur. */
let moteurs: EngineStatus[] = [];
/** Les octets recus par moteur en cours de telechargement. */
const avancement = new Map<string, { recus: number; attendus: number }>();
/** Les moteurs dont l'installation a echoue, avec leur message. */
const echecs = new Map<string, string>();
/** Les moteurs dont une installation est en cours. */
const enCours = new Set<string>();

/** Ce qu'un moteur optionnel apporte, en une phrase. */
const APPORT: Record<string, string> = {
  libreoffice:
    "Ouvre les .doc de Word 97-2003, les classeurs et les présentations, et produit " +
    "des PDF fidèles à la mise en page d'origine.",
};

/** Vrai quand l'ecran de preparation a deja ete montre une fois. */
function dejaVu(): boolean {
  try {
    return window.localStorage.getItem(CLE_VU) === "1";
  } catch {
    // Un stockage indisponible ne doit pas empecher l'application de demarrer.
    return false;
  }
}

/** Retient que l'ecran a ete montre. */
function marquerVu(): void {
  try {
    window.localStorage.setItem(CLE_VU, "1");
  } catch {
    // Sans stockage, l'ecran reapparaitra : genant, jamais bloquant.
  }
}

/**
 * Les moteurs que la preparation installe d'office.
 *
 * Les moteurs optionnels en sont exclus : le moteur haute fidelite pese 1,5 Go
 * une fois deplie, et l'imposer au premier lancement ferait fuir la plupart des
 * gens pour une capacite dont beaucoup n'auront jamais besoin.
 */
function moteursEssentiels(): EngineStatus[] {
  return moteurs.filter((moteur) => !moteur.optional);
}

/** Les moteurs proposes, jamais imposes. */
function moteursOptionnels(): EngineStatus[] {
  return moteurs.filter((moteur) => moteur.optional);
}

/** La somme de ce qui reste a telecharger d'office. */
function totalATelecharger(): number {
  return moteursEssentiels()
    .filter((moteur) => !moteur.installed)
    .reduce((somme, moteur) => somme + moteur.downloadBytes, 0);
}

/** La fraction deja recue, de 0 a 1, sur les seuls moteurs requis. */
function fractionGlobale(): number {
  const essentiels = moteursEssentiels();
  const total = essentiels.reduce((somme, moteur) => somme + moteur.downloadBytes, 0);
  if (total === 0) return 1;
  const recus = essentiels.reduce((somme, moteur) => {
    if (moteur.installed) return somme + moteur.downloadBytes;
    return somme + (avancement.get(moteur.id)?.recus ?? 0);
  }, 0);
  return Math.min(recus / total, 1);
}

/** Vrai tant qu'un moteur requis s'installe. */
function preparationEnCours(): boolean {
  return moteursEssentiels().some((moteur) => enCours.has(moteur.id));
}

/** La carte d'un moteur requis. */
function carteRequise(moteur: EngineStatus): string {
  const echec = echecs.get(moteur.id);
  const progression = avancement.get(moteur.id);
  const installe = enCours.has(moteur.id);

  const etat = moteur.installed ? "ready" : installe ? "installing" : "missing";

  const detail = moteur.installed
    ? `Prêt — ${escapeHtml(moteur.version ?? "")}`
    : echec
      ? escapeHtml(echec)
      : progression
        ? `${formatBytes(progression.recus)} sur ${formatBytes(progression.attendus)}`
        : `${formatBytes(moteur.downloadBytes)} à télécharger, ${formatBytes(moteur.installedBytes)} sur le disque`;

  return `<div class="engine-card" data-state="${etat}">
    <div class="engine-card__head">
      <span class="engine-card__dot"></span>
      <span class="engine-card__name">${escapeHtml(moteur.label)}</span>
      <span class="engine-card__badge">${moteur.installed ? "installé" : "requis"}</span>
    </div>
    <p class="engine-card__note">${detail}</p>
    ${
      installe && !moteur.installed
        ? `<div class="progress"><div class="progress__bar" data-moteur="${escapeHtml(moteur.id)}"></div></div>`
        : ""
    }
  </div>`;
}

/**
 * La carte d'un moteur optionnel.
 *
 * Elle dit ce qu'il apporte, ce qu'il coute, et n'engage rien tant qu'on n'a
 * pas clique sur son propre bouton.
 */
function carteOptionnelle(moteur: EngineStatus): string {
  const echec = echecs.get(moteur.id);
  const progression = avancement.get(moteur.id);
  const installe = enCours.has(moteur.id);
  const apport = APPORT[moteur.id] ?? "";

  if (moteur.installed) {
    return `<div class="engine-card" data-state="ready">
      <div class="engine-card__head">
        <span class="engine-card__dot"></span>
        <span class="engine-card__name">${escapeHtml(moteur.label)}</span>
        <span class="engine-card__badge">actif</span>
      </div>
      <p class="engine-card__note">${escapeHtml(apport)}</p>
    </div>`;
  }

  if (installe) {
    const pourcent = progression
      ? Math.round((progression.recus / Math.max(progression.attendus, 1)) * 100)
      : 0;
    return `<div class="engine-card" data-state="installing">
      <div class="engine-card__head">
        <span class="engine-card__dot"></span>
        <span class="engine-card__name">${escapeHtml(moteur.label)}</span>
        <span class="engine-card__badge">${pourcent} %</span>
      </div>
      <p class="engine-card__note">
        ${
          progression
            ? `${formatBytes(progression.recus)} sur ${formatBytes(progression.attendus)}`
            : "Téléchargement en cours"
        }
      </p>
      <div class="progress"><div class="progress__bar" data-moteur="${escapeHtml(moteur.id)}"></div></div>
      <p class="engine-card__note">Vous pouvez entrer dans Derovia, il continue en arrière-plan.</p>
    </div>`;
  }

  return `<div class="engine-card" data-state="missing">
    <div class="engine-card__head">
      <span class="engine-card__dot"></span>
      <span class="engine-card__name">${escapeHtml(moteur.label)}</span>
      <span class="engine-card__badge">optionnel</span>
    </div>
    <p class="engine-card__note">${echec ? escapeHtml(echec) : escapeHtml(apport)}</p>
    <p class="engine-card__note">
      ${formatBytes(moteur.downloadBytes)} à télécharger, ${formatBytes(moteur.installedBytes)} sur le disque.
    </p>
    <button class="btn btn--secondary btn--sm" type="button"
            data-installer="${escapeHtml(moteur.id)}">
      ${echec ? "Réessayer" : "Ajouter ce moteur"}
    </button>
  </div>`;
}

/** Dessine l'ecran. */
function render(racine: HTMLElement, onTermine: () => void): void {
  const tousPrets = moteursEssentiels().every((moteur) => moteur.installed);
  const travailEnCours = preparationEnCours();
  const optionnels = moteursOptionnels();

  racine.innerHTML = `
    <div class="launcher__topbar"></div>
    <div class="launcher__body">
      <div class="launcher__hero">
        <div class="launcher__mark" aria-hidden="true">D</div>
        <h1 class="launcher__wordmark">Préparation</h1>
        <p class="launcher__tagline">
          ${
            tousPrets
              ? "Tout est prêt. Derovia dispose de ses moteurs de conversion."
              : `Derovia utilise des moteurs de conversion externes pour préserver
                 tableaux, notes et styles. ${escapeHtml(formatBytes(totalATelecharger()))} à récupérer, une seule fois.`
          }
        </p>
      </div>

      <div class="launcher__section-label">Moteurs</div>
      <div class="setup__list">${moteursEssentiels().map(carteRequise).join("")}</div>

      ${
        travailEnCours
          ? `<div class="setup__global">
              <div class="progress"><div class="progress__bar" id="setup-global"></div></div>
              <p class="engine-card__note" id="setup-global-note"></p>
            </div>`
          : ""
      }

      <div class="setup__actions">
        ${
          tousPrets
            ? `<button class="btn btn--primary" type="button" id="setup-entrer">Entrer dans Derovia</button>`
            : travailEnCours
              ? `<button class="btn btn--secondary" type="button" id="setup-entrer">
                   Continuer pendant le téléchargement
                 </button>`
              : `<button class="btn btn--primary" type="button" id="setup-installer">
                   Installer les moteurs
                 </button>
                 <button class="btn btn--ghost" type="button" id="setup-plus-tard">Plus tard</button>`
        }
      </div>

      ${
        tousPrets || travailEnCours
          ? ""
          : `<p class="setup__note">
              Sans ces moteurs, la suite fonctionne déjà : la conversion passe par
              le moteur intégré, qui rend la structure des documents mais perd les
              tableaux et les notes.
            </p>`
      }

      ${
        optionnels.length === 0
          ? ""
          : `<div class="launcher__section-label setup__optional-label">Pour aller plus loin</div>
             <div class="setup__list">${optionnels.map(carteOptionnelle).join("")}</div>
             <p class="setup__note">
               Celui-ci ne s'installe que si vous le demandez, et peut être ajouté
               plus tard depuis le Convertisseur.
             </p>`
      }
    </div>
  `;

  racine.querySelector("#setup-installer")?.addEventListener("click", () => {
    void installerRequis(racine, onTermine);
  });
  for (const id of ["setup-entrer", "setup-plus-tard"]) {
    racine.querySelector(`#${id}`)?.addEventListener("click", () => {
      marquerVu();
      onTermine();
    });
  }
  for (const bouton of racine.querySelectorAll<HTMLButtonElement>("[data-installer]")) {
    bouton.addEventListener("click", () => {
      const id = bouton.dataset["installer"];
      if (id) void installerUn(id, racine, onTermine);
    });
  }

  majBarres(racine);
}

/** Met les barres a jour sans redessiner : un rendu par paquet ferait clignoter. */
function majBarres(racine: HTMLElement): void {
  for (const [id, { recus, attendus }] of avancement) {
    const fraction = attendus > 0 ? Math.min(recus / attendus, 1) : 0;
    racine
      .querySelector<HTMLElement>(`[data-moteur="${CSS.escape(id)}"]`)
      ?.style.setProperty("--progress", `${(fraction * 100).toFixed(1)}%`);
  }

  const globale = racine.querySelector<HTMLElement>("#setup-global");
  const fraction = fractionGlobale();
  globale?.style.setProperty("--progress", `${(fraction * 100).toFixed(1)}%`);

  const note = racine.querySelector<HTMLElement>("#setup-global-note");
  if (note) {
    note.textContent = `${Math.round(fraction * 100)} % — vous pouvez continuer, le téléchargement se poursuit.`;
  }
}

/** Installe un moteur en suivant sa progression, et rend la main apres coup. */
async function installerAvecSuivi(
  id: string,
  racine: HTMLElement,
  redessiner: () => void,
): Promise<void> {
  const moteur = moteurs.find((candidat) => candidat.id === id);
  if (!moteur || moteur.installed) return;

  enCours.add(id);
  echecs.delete(id);
  avancement.set(id, { recus: 0, attendus: moteur.downloadBytes });
  redessiner();

  const cesser = await suivreInstallation((progression) => {
    if (progression.id !== id) return;
    avancement.set(id, { recus: progression.recus, attendus: progression.attendus });
    majBarres(racine);
  });

  try {
    await installerMoteur(id);
  } catch (erreur: unknown) {
    // Un moteur qui echoue ne doit pas empecher les autres, ni rester muet.
    echecs.set(
      id,
      erreur instanceof Error ? erreur.message : String(erreur ?? "Installation impossible"),
    );
  } finally {
    cesser();
    enCours.delete(id);
    avancement.delete(id);
    moteurs = await moteursStatut().catch(() => moteurs);
    redessiner();
  }
}

/** Installe les moteurs requis, l'un apres l'autre — et eux seuls. */
async function installerRequis(racine: HTMLElement, onTermine: () => void): Promise<void> {
  const redessiner = (): void => {
    render(racine, onTermine);
  };
  const aFaire = moteursEssentiels()
    .filter((moteur) => !moteur.installed)
    .map((moteur) => moteur.id);

  for (const id of aFaire) {
    await installerAvecSuivi(id, racine, redessiner);
  }
  marquerVu();
  redessiner();
}

/** Installe un moteur optionnel, a la demande expresse. */
async function installerUn(id: string, racine: HTMLElement, onTermine: () => void): Promise<void> {
  await installerAvecSuivi(id, racine, () => {
    render(racine, onTermine);
  });
}

/**
 * Montre l'ecran de preparation si besoin, puis rend la main.
 *
 * `onTermine` est appele quand l'utilisateur entre dans la suite — que les
 * moteurs soient installes ou non.
 */
export async function preparer(racine: HTMLElement, onTermine: () => void): Promise<void> {
  try {
    moteurs = await moteursStatut();
  } catch {
    // Hors de la fenetre Tauri, il n'y a rien a installer : on passe.
    onTermine();
    return;
  }

  const manquants = moteursEssentiels().some((moteur) => !moteur.installed);
  if (!manquants || dejaVu()) {
    onTermine();
    return;
  }

  racine.hidden = false;
  render(racine, onTermine);
}
