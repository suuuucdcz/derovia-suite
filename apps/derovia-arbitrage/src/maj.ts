/**
 * Les mises a jour de l'application.
 *
 * Sans ce mecanisme, une installation reste figee sur la version telechargee :
 * un correctif de securite ou une reparation ne parvient jamais aux personnes
 * qui ont deja installe l'outil.
 *
 * Le paquet recu est **verifie par signature** avant d'etre applique. La cle
 * publique est compilee dans l'application ; sans elle, quiconque interceptant
 * le reseau pourrait livrer son propre executable a la place de la mise a jour.
 *
 * Rien n'est impose : la notification est discrete, et l'utilisateur decide du
 * moment. Une mise a jour qui s'applique de force pendant qu'on travaille est
 * une interruption, pas un service.
 */

import { inTauri } from "./api";
import { escapeHtml } from "./dom";
import { formatBytes } from "./format";

/** L'element de notification, cree a la demande. */
let bandeau: HTMLElement | null = null;

/** Prepare le bandeau flottant et le renvoie. */
function obtenirBandeau(): HTMLElement {
  if (bandeau) return bandeau;
  bandeau = document.createElement("div");
  bandeau.className = "maj-toast";
  bandeau.hidden = true;
  document.body.append(bandeau);
  return bandeau;
}

/** Masque la notification. */
function masquer(): void {
  const element = obtenirBandeau();
  element.hidden = true;
  element.innerHTML = "";
}

/**
 * Surveille les mises a jour et propose de les appliquer.
 *
 * Sans effet hors de la fenetre Tauri : le serveur de developpement n'a pas de
 * version installee a remplacer.
 */
export async function surveillerMiseAJour(): Promise<void> {
  if (!inTauri()) return;

  const { check } = await import("@tauri-apps/plugin-updater");

  let disponible;
  try {
    disponible = await check();
  } catch (erreur: unknown) {
    // Une verification qui echoue — reseau coupe, serveur indisponible — ne
    // doit jamais gener l'utilisation de l'outil. On note, et on continue.
    console.warn("[Derovia] vérification des mises à jour impossible :", erreur);
    return;
  }

  if (!disponible) return;

  const element = obtenirBandeau();
  element.hidden = false;
  element.innerHTML = `
    <div class="maj-toast__texte">
      <div class="maj-toast__titre">Derovia ${escapeHtml(disponible.version)} est disponible</div>
      <div class="maj-toast__note" id="maj-note">
        ${disponible.date ? `Publiée le ${escapeHtml(disponible.date.slice(0, 10))}` : "Une nouvelle version est prête"}
      </div>
      <div class="progress" id="maj-progress" hidden>
        <div class="progress__bar" id="maj-barre"></div>
      </div>
    </div>
    <div class="maj-toast__actions">
      <button class="btn btn--ghost btn--sm" type="button" id="maj-plus-tard">Plus tard</button>
      <button class="btn btn--primary btn--sm" type="button" id="maj-installer">Installer</button>
    </div>
  `;

  element.querySelector("#maj-plus-tard")?.addEventListener("click", masquer);
  element.querySelector("#maj-installer")?.addEventListener("click", () => {
    void appliquer(disponible);
  });
}

/** Ce que le greffon de mise a jour expose et que nous utilisons. */
interface MiseAJour {
  version: string;
  downloadAndInstall: (
    onEvent: (evenement: {
      event: string;
      data?: { contentLength?: number; chunkLength?: number };
    }) => void,
  ) => Promise<void>;
}

/** Telecharge, verifie et applique la mise a jour, puis relance. */
async function appliquer(disponible: MiseAJour): Promise<void> {
  const element = obtenirBandeau();
  const bouton = element.querySelector<HTMLButtonElement>("#maj-installer");
  const note = element.querySelector<HTMLElement>("#maj-note");
  const barreConteneur = element.querySelector<HTMLElement>("#maj-progress");
  const barre = element.querySelector<HTMLElement>("#maj-barre");

  if (bouton) {
    bouton.disabled = true;
    bouton.textContent = "Installation…";
  }
  element.querySelector<HTMLButtonElement>("#maj-plus-tard")?.setAttribute("disabled", "");
  if (barreConteneur) barreConteneur.hidden = false;

  let total = 0;
  let recus = 0;

  try {
    await disponible.downloadAndInstall((evenement) => {
      if (evenement.event === "Started") {
        total = evenement.data?.contentLength ?? 0;
      } else if (evenement.event === "Progress") {
        recus += evenement.data?.chunkLength ?? 0;
        const fraction = total > 0 ? Math.min(recus / total, 1) : 0;
        barre?.style.setProperty("--progress", `${(fraction * 100).toFixed(1)}%`);
        if (note) {
          note.textContent =
            total > 0
              ? `${formatBytes(recus)} sur ${formatBytes(total)}`
              : `${formatBytes(recus)} reçus`;
        }
      } else if (evenement.event === "Finished" && note) {
        note.textContent = "Vérification de la signature…";
      }
    });

    if (note) note.textContent = "Installée. Redémarrage…";
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch (erreur: unknown) {
    // L'echec est montre plutot qu'avale : un bouton qui ne fait visiblement
    // rien laisse croire a une application cassee.
    if (note) {
      note.textContent =
        erreur instanceof Error ? erreur.message : "La mise à jour n'a pas pu être appliquée.";
      note.dataset["tone"] = "negative";
    }
    if (barreConteneur) barreConteneur.hidden = true;
    if (bouton) {
      bouton.disabled = false;
      bouton.textContent = "Réessayer";
    }
    element.querySelector<HTMLButtonElement>("#maj-plus-tard")?.removeAttribute("disabled");
  }
}
