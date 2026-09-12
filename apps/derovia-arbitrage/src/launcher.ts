/**
 * L'ecran de lancement de la suite.
 *
 * Il fait trois choses : presenter la marque, lister les outils, et dire quel
 * compte est connecte. Il ne connait rien du metier d'aucun outil — il recoit
 * une fonction a appeler quand l'utilisateur en ouvre un.
 *
 * Il ne connait rien de l'authentification : on n'atteint cet ecran qu'une fois
 * la porte franchie. Il se contente d'afficher qui est connecte.
 */

import { compteActuel, deconnecter, surChangementDeCompte, type Compte } from "./auth";
import { escapeHtml } from "./dom";
import { SUITE_TOOLS, SUITE_VERSION, type SuiteTool } from "./suite";

type OuvrirOutil = (tool: SuiteTool) => void;

// --- Ecran de lancement -----------------------------------------------------

function toolCardMarkup(tool: SuiteTool): string {
  const action = tool.available ? "Ouvrir →" : "Bientôt";
  return `<button class="tool-card" type="button" data-tool="${escapeHtml(tool.id)}"
      ${tool.available ? "" : "disabled"}>
    <span class="tool-card__glyph" aria-hidden="true">${escapeHtml(tool.glyph)}</span>
    <span class="tool-card__name">${escapeHtml(tool.name)}</span>
    <span class="tool-card__desc">${escapeHtml(tool.description)}</span>
    <span class="tool-card__action">${escapeHtml(action)}</span>
  </button>`;
}

function navItemMarkup(tool: SuiteTool, activeId: string): string {
  const soon = tool.available ? "" : `<span class="nav-item__soon">Bientôt</span>`;
  const current = tool.id === activeId ? ` aria-current="page"` : "";
  return `<button class="nav-item" type="button" data-tool="${escapeHtml(tool.id)}"
      ${tool.available ? "" : "disabled"}${current}>
    <span class="nav-item__glyph" aria-hidden="true">${escapeHtml(tool.glyph)}</span>
    ${escapeHtml(tool.name)}${soon}
  </button>`;
}

// --- La barre de compte ----------------------------------------------------

/** Le compte connecte, ou `null`. */
let compte: Compte | null = null;

/**
 * Met la barre d'accueil en accord avec l'etat de connexion.
 *
 * Il n'y a plus d'entree « se connecter » ici : on n'atteint cet ecran qu'une
 * fois passe la porte. La barre n'a donc qu'a dire qui est la, et offrir de
 * partir.
 */
function rendreCompte(): void {
  const barre = document.querySelector<HTMLElement>(".launcher__topbar");
  if (!barre) return;

  if (!compte) {
    barre.innerHTML = "";
    return;
  }

  barre.innerHTML = `
    <span class="launcher__account">Connecté en tant que <strong>${escapeHtml(compte.email)}</strong></span>
    <button class="btn btn--secondary" type="button" id="auth-signout">Se déconnecter</button>`;
  barre.querySelector("#auth-signout")?.addEventListener("click", () => {
    void deconnecter();
  });
}

function wireAuth(): void {
  // Supabase peut invalider ou renouveler une session sans intervention :
  // l'interface doit suivre plutot que refleter un etat perime.
  surChangementDeCompte((nouveau) => {
    const avait = compte !== null;
    compte = nouveau;
    rendreCompte();
    // Une session qui disparait — deconnexion volontaire, ou jeton revoque —
    // doit ramener a la porte. Laisser la suite ouverte donnerait acces a des
    // outils au nom de quelqu'un qui n'est plus la.
    if (avait && !nouveau) window.location.reload();
  });

  void compteActuel().then((existant) => {
    compte = existant;
    rendreCompte();
  });
}


// --- Navigation entre l'accueil et l'outil ----------------------------------

function showLauncher(): void {
  const launcher = document.querySelector<HTMLElement>("#launcher");
  const workspace = document.querySelector<HTMLElement>("#workspace");
  if (launcher) launcher.hidden = false;
  if (workspace) workspace.hidden = true;
}

function showWorkspace(): void {
  const launcher = document.querySelector<HTMLElement>("#launcher");
  const workspace = document.querySelector<HTMLElement>("#workspace");
  if (launcher) launcher.hidden = true;
  if (workspace) workspace.hidden = false;
}

/**
 * Construit l'ecran de lancement et branche la navigation.
 *
 * `ouvrirOutil` est appelee quand l'utilisateur ouvre un outil disponible ;
 * c'est a l'appelant de demarrer l'espace de travail correspondant.
 */
export function mountLauncher(ouvrirOutil: OuvrirOutil): void {
  const grid = document.querySelector<HTMLElement>("#tool-grid");
  const nav = document.querySelector<HTMLElement>("#sidebar-nav");
  const footer = document.querySelector<HTMLElement>("#launcher-footer");
  const active = SUITE_TOOLS.find((tool) => tool.available);

  if (grid) grid.innerHTML = SUITE_TOOLS.map(toolCardMarkup).join("");
  if (nav) {
    nav.insertAdjacentHTML(
      "beforeend",
      SUITE_TOOLS.map((tool) => navItemMarkup(tool, active?.id ?? "")).join(""),
    );
  }
  if (footer) {
    footer.textContent = `Derovia ${SUITE_VERSION} — vos fichiers sont traités sur cette machine et n'en sortent pas.`;
  }

  function setActiveNav(toolId: string): void {
    if (!nav) return;
    for (const item of nav.querySelectorAll<HTMLElement>(".nav-item")) {
      if (item.dataset["tool"] === toolId) {
        item.setAttribute("aria-current", "page");
      } else {
        item.removeAttribute("aria-current");
      }
    }
  }

  grid?.addEventListener("click", (event) => {
    const id = (event.target as HTMLElement | null)?.closest<HTMLElement>("[data-tool]")?.dataset[
      "tool"
    ];
    const tool = SUITE_TOOLS.find((candidate) => candidate.id === id);
    if (!tool?.available) return;
    setActiveNav(tool.id);
    showWorkspace();
    ouvrirOutil(tool);
  });

  nav?.addEventListener("click", (event) => {
    const id = (event.target as HTMLElement | null)?.closest<HTMLElement>("[data-tool]")?.dataset[
      "tool"
    ];
    const tool = SUITE_TOOLS.find((candidate) => candidate.id === id);
    if (!tool?.available) return;
    setActiveNav(tool.id);
    ouvrirOutil(tool);
  });

  document.querySelector("#back-to-launcher")?.addEventListener("click", showLauncher);

  wireAuth();
}
