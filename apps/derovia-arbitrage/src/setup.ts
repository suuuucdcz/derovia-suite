/**
 * L'ecran de preparation du premier lancement.
 *
 * Derovia s'appuie sur des moteurs externes — Pandoc aujourd'hui — trop lourds
 * pour tenir dans l'installeur. Cet ecran les installe en une fois, avec une
 * barre de progression.
 *
 * **Il ne prend jamais l'application en otage.** Sur une connexion lente, ces
 * telechargements durent plusieurs minutes : on peut entrer dans la suite
 * pendant qu'ils se poursuivent en arriere-plan, et chaque outil s'active des
 * que son moteur est pret. Un ecran bloquant aurait ete plus simple a ecrire,
 * et insupportable a utiliser.
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
/** Vrai tant qu'une installation tourne. */
let enCours = false;

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
 * Les moteurs que la preparation installe.
 *
 * Les moteurs optionnels en sont exclus : le moteur haute fidelite pese 1,5 Go
 * une fois deplie, et l'imposer au premier lancement ferait fuir la plupart des
 * gens pour une capacite dont beaucoup n'auront jamais besoin. Il est propose
 * plus tard, la ou il change vraiment le resultat.
 */
function moteursEssentiels(): EngineStatus[] {
  return moteurs.filter((moteur) => !moteur.optional);
}

/** La somme de ce qui reste a telecharger. */
function totalATelecharger(): number {
  return moteursEssentiels()
    .filter((moteur) => !moteur.installed)
    .reduce((somme, moteur) => somme + moteur.downloadBytes, 0);
}

/** La fraction globale deja recue, de 0 a 1. */
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

/** La ligne d'un moteur dans la liste. */
function ligneMoteur(moteur: EngineStatus): string {
  const echec = echecs.get(moteur.id);
  const encours = avancement.get(moteur.id);

  const etat = moteur.installed
    ? "ready"
    : echec
      ? "missing"
      : encours
        ? "installing"
        : "missing";

  const detail = moteur.installed
    ? `Prêt — ${escapeHtml(moteur.version ?? "")}`
    : echec
      ? escapeHtml(echec)
      : encours
        ? `${formatBytes(encours.recus)} sur ${formatBytes(encours.attendus)}`
        : `${formatBytes(moteur.downloadBytes)} à télécharger, ${formatBytes(moteur.installedBytes)} sur le disque`;

  return `<div class="engine-card" data-state="${etat}">
    <div class="engine-card__head">
      <span class="engine-card__dot"></span>
      <span class="engine-card__name">${escapeHtml(moteur.label)}</span>
      <span class="engine-card__badge">${moteur.installed ? "installé" : "requis"}</span>
    </div>
    <p class="engine-card__note">${detail}</p>
    ${
      encours && !moteur.installed
        ? `<div class="progress"><div class="progress__bar" data-moteur="${escapeHtml(moteur.id)}"></div></div>`
        : ""
    }
  </div>`;
}

/** Dessine l'ecran. */
function render(racine: HTMLElement, onTermine: () => void): void {
  const manquants = moteursEssentiels().filter((moteur) => !moteur.installed);
  const tousPrets = manquants.length === 0;

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
      <div class="setup__list">${moteursEssentiels().map(ligneMoteur).join("")}</div>

      ${
        enCours
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
            : enCours
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
        tousPrets || enCours
          ? ""
          : `<p class="setup__note">
              Sans ces moteurs, la suite fonctionne déjà : la conversion passe par
              le moteur intégré, qui rend la structure des documents mais perd les
              tableaux et les notes.
            </p>`
      }
    </div>
  `;

  racine.querySelector("#setup-installer")?.addEventListener("click", () => {
    void installerTout(racine, onTermine);
  });
  for (const id of ["setup-entrer", "setup-plus-tard"]) {
    racine.querySelector(`#${id}`)?.addEventListener("click", () => {
      marquerVu();
      onTermine();
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
  if (note) note.textContent = `${Math.round(fraction * 100)} % — vous pouvez continuer, le téléchargement se poursuit.`;
}

/** Installe tous les moteurs manquants, l'un apres l'autre. */
async function installerTout(racine: HTMLElement, onTermine: () => void): Promise<void> {
  enCours = true;
  echecs.clear();
  render(racine, onTermine);

  const cesser = await suivreInstallation((progression) => {
    avancement.set(progression.id, {
      recus: progression.recus,
      attendus: progression.attendus,
    });
    majBarres(racine);
  });

  try {
    for (const moteur of moteurs.filter((candidat) => !candidat.installed)) {
      avancement.set(moteur.id, { recus: 0, attendus: moteur.downloadBytes });
      render(racine, onTermine);
      try {
        await installerMoteur(moteur.id);
      } catch (erreur: unknown) {
        // Un moteur qui echoue ne doit pas empecher les suivants.
        echecs.set(
          moteur.id,
          erreur instanceof Error ? erreur.message : String(erreur ?? "Installation impossible"),
        );
      }
      avancement.delete(moteur.id);
      moteurs = await moteursStatut().catch(() => moteurs);
      render(racine, onTermine);
    }
  } finally {
    cesser();
    enCours = false;
    marquerVu();
    render(racine, onTermine);
  }
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
