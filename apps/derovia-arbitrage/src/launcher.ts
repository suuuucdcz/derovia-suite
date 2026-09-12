/**
 * L'ecran de lancement de la suite.
 *
 * Il fait trois choses : presenter la marque, lister les outils, et ouvrir la
 * fenetre de compte. Il ne connait rien du metier d'aucun outil — il recoit une
 * fonction a appeler quand l'utilisateur en ouvre un.
 *
 * La fenetre de compte s'adresse a Supabase. Le mot de passe saisi part
 * directement au client Supabase et n'est jamais conserve ici : aucune variable
 * de ce module ne le retient, et le champ est vide des la fermeture.
 */

import {
  authDisponible,
  compteActuel,
  connecter,
  deconnecter,
  inscrire,
  surChangementDeCompte,
  type Compte,
} from "./auth";
import { escapeHtml } from "./dom";
import { SUITE_TOOLS, SUITE_VERSION, type SuiteTool } from "./suite";

type OuvrirOutil = (tool: SuiteTool) => void;

const AUTH_TEXTS = {
  signin: {
    title: "Se connecter",
    lede: "Retrouvez vos scénarios sur tous vos postes.",
    submit: "Se connecter",
  },
  signup: {
    title: "Créer un compte",
    lede: "Un compte Derovia vous suivra sur tous les outils de la suite.",
    submit: "Créer mon compte",
  },
} as const;

type AuthMode = keyof typeof AUTH_TEXTS;

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

// --- Fenetre de compte ------------------------------------------------------

const modal = document.querySelector<HTMLElement>("#auth-modal");

/** Le compte connecte, ou `null`. */
let compte: Compte | null = null;

/** Affiche un message dans la fenetre de compte. */
function afficherMessage(texte: string, ton: "negative" | "positive"): void {
  const zone = document.querySelector<HTMLElement>("#auth-message");
  if (!zone) return;
  zone.textContent = texte;
  zone.dataset["tone"] = ton;
  zone.hidden = false;
}

/** Efface le message de la fenetre de compte. */
function effacerMessage(): void {
  const zone = document.querySelector<HTMLElement>("#auth-message");
  if (!zone) return;
  zone.hidden = true;
  zone.textContent = "";
}

/**
 * Met la barre d'accueil en accord avec l'etat de connexion.
 *
 * Connecte, elle montre l'adresse et propose de se deconnecter ; deconnecte,
 * elle propose les deux entrees habituelles.
 */
function rendreCompte(): void {
  const barre = document.querySelector<HTMLElement>(".launcher__topbar");
  if (!barre) return;

  if (compte) {
    barre.innerHTML = `
      <span class="launcher__account">Connecté en tant que <strong>${escapeHtml(compte.email)}</strong></span>
      <button class="btn btn--secondary" type="button" id="auth-signout">Se déconnecter</button>`;
    barre.querySelector("#auth-signout")?.addEventListener("click", () => {
      void deconnecter();
    });
    return;
  }

  // Sans configuration Supabase, proposer un formulaire qui echouerait a coup
  // sur serait pire que de ne rien proposer.
  barre.innerHTML = authDisponible()
    ? `<button class="btn btn--ghost" type="button" data-auth="signin">Se connecter</button>
       <button class="btn btn--primary" type="button" data-auth="signup">Créer un compte</button>`
    : "";
  wireBoutonsCompte();
}

/** Fait glisser la pastille du selecteur sous l'onglet actif. */
function positionAuthThumb(): void {
  const thumb = document.querySelector<HTMLElement>("#auth-thumb");
  const selected = document.querySelector<HTMLElement>('#auth-switch button[aria-selected="true"]');
  if (!thumb || !selected) return;
  thumb.style.width = `${selected.offsetWidth}px`;
  thumb.style.transform = `translateX(${selected.offsetLeft - 2}px)`;
}

function applyAuthMode(mode: AuthMode): void {
  const texts = AUTH_TEXTS[mode];
  const title = document.querySelector<HTMLElement>("#auth-title");
  const lede = document.querySelector<HTMLElement>("#auth-lede");
  const submit = document.querySelector<HTMLElement>("#auth-submit");
  const nameField = document.querySelector<HTMLElement>("#auth-name-field");

  if (title) title.textContent = texts.title;
  if (lede) lede.textContent = texts.lede;
  if (submit) submit.textContent = texts.submit;
  if (nameField) nameField.hidden = mode !== "signup";

  // Le gestionnaire de mots de passe propose un mot de passe fort a
  // l'inscription, et le mot de passe enregistre a la connexion.
  const motDePasse = document.querySelector<HTMLInputElement>("#auth-password");
  if (motDePasse) {
    motDePasse.autocomplete = mode === "signup" ? "new-password" : "current-password";
  }
  effacerMessage();

  for (const tab of document.querySelectorAll<HTMLElement>("#auth-switch button")) {
    tab.setAttribute("aria-selected", String(tab.dataset["mode"] === mode));
  }
  positionAuthThumb();
}

function openAuth(mode: AuthMode): void {
  if (!modal) return;
  modal.hidden = false;
  // La pastille se mesure une fois la fenetre visible : masquee, tout element
  // a une largeur nulle et la pastille se collerait a gauche.
  applyAuthMode(mode);
  modal.querySelector<HTMLInputElement>('input[type="email"]')?.focus();
}

function closeAuth(): void {
  if (!modal) return;
  modal.hidden = true;
  // Le formulaire est vide : le mot de passe ne doit pas rester dans le DOM.
  document.querySelector<HTMLFormElement>("#auth-form")?.reset();
  effacerMessage();
}

/** Branche les deux entrees de la barre d'accueil. */
function wireBoutonsCompte(): void {
  for (const button of document.querySelectorAll<HTMLElement>("[data-auth]")) {
    button.addEventListener("click", () => {
      const mode = button.dataset["auth"];
      if (mode === "signin" || mode === "signup") openAuth(mode);
    });
  }
}

/** Envoie le formulaire a Supabase et rend compte du resultat. */
async function soumettreFormulaire(): Promise<void> {
  const submit = document.querySelector<HTMLButtonElement>("#auth-submit");
  const email = document.querySelector<HTMLInputElement>("#auth-email")?.value.trim() ?? "";
  const motDePasse = document.querySelector<HTMLInputElement>("#auth-password")?.value ?? "";
  const mode =
    document
      .querySelector<HTMLElement>('#auth-switch button[aria-selected="true"]')
      ?.dataset["mode"] === "signup"
      ? "signup"
      : "signin";

  if (!submit) return;
  const libelle = submit.textContent ?? "Continuer";
  submit.disabled = true;
  submit.textContent = mode === "signup" ? "Création…" : "Connexion…";
  effacerMessage();

  const resultat = mode === "signup"
    ? await inscrire(email, motDePasse)
    : await connecter(email, motDePasse);

  submit.disabled = false;
  submit.textContent = libelle;

  switch (resultat.etat) {
    case "connecte":
      // `surChangementDeCompte` se charge de rafraichir la barre d'accueil.
      closeAuth();
      break;
    case "confirmation_requise":
      afficherMessage(
        `Compte créé. Ouvrez le lien envoyé à ${resultat.email} pour l'activer, puis connectez-vous.`,
        "positive",
      );
      break;
    case "erreur":
      afficherMessage(resultat.message, "negative");
      break;
  }
}

function wireAuth(): void {
  wireBoutonsCompte();

  for (const target of document.querySelectorAll<HTMLElement>("[data-close-modal]")) {
    target.addEventListener("click", closeAuth);
  }

  document.querySelector("#auth-switch")?.addEventListener("click", (event) => {
    const mode = (event.target as HTMLElement | null)?.closest<HTMLElement>("[data-mode]")?.dataset[
      "mode"
    ];
    if (mode === "signin" || mode === "signup") applyAuthMode(mode);
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && modal && !modal.hidden) closeAuth();
  });

  document.querySelector<HTMLFormElement>("#auth-form")?.addEventListener("submit", (event) => {
    event.preventDefault();
    void soumettreFormulaire();
  });

  window.addEventListener("resize", () => {
    if (modal && !modal.hidden) positionAuthThumb();
  });

  // Supabase peut invalider ou renouveler une session sans intervention :
  // l'interface doit suivre plutot que refleter un etat perime.
  surChangementDeCompte((nouveau) => {
    compte = nouveau;
    rendreCompte();
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
