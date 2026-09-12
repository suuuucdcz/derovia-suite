/**
 * La porte d'entree de la suite.
 *
 * Derovia ne s'ouvre qu'a un compte. Ce n'est pas une formalite administrative :
 * les scenarios enregistres appartiennent a quelqu'un, et un outil qui laisse
 * entrer sans savoir qui n'a nulle part ou les ranger.
 *
 * L'ecran occupe toute la fenetre et ne se ferme pas. Une fenetre modale
 * aurait laisse entrevoir une suite qu'on ne peut pas utiliser, avec une croix
 * dans le coin pour s'en approcher : une porte qu'on peut contourner n'est pas
 * une porte.
 *
 * Le mot de passe saisi part directement au client Supabase. Aucune variable de
 * ce module ne le retient, et le formulaire est vide des qu'on entre.
 */

import {
  authDisponible,
  compteActuel,
  connecter,
  inscrire,
  surChangementDeCompte,
  type Compte,
} from "./auth";

/** Les deux intentions possibles devant la porte. */
type Mode = "signin" | "signup";

const TEXTES = {
  signin: {
    titre: "Content de vous revoir",
    lede: "Vos scénarios vous attendent.",
    action: "Se connecter",
    occupe: "Connexion…",
  },
  signup: {
    titre: "Bienvenue dans Derovia",
    lede: "Un seul compte pour tous les outils de la suite.",
    action: "Créer mon compte",
    occupe: "Création…",
  },
} as const;

/** Le mode affiche, conserve d'un rendu a l'autre. */
let mode: Mode = "signin";

/** Dessine la porte. */
function render(racine: HTMLElement, onConnecte: () => void): void {
  const textes = TEXTES[mode];

  // Sans configuration, aucun compte n'est possible — et donc aucun acces. Le
  // dire franchement vaut mieux qu'un formulaire qui echouerait a chaque essai.
  if (!authDisponible()) {
    racine.innerHTML = `
      <div class="launcher__body">
        <div class="porte">
          <div class="launcher__mark" aria-hidden="true">D</div>
          <h1 class="porte__titre">Derovia ne peut pas démarrer</h1>
          <p class="porte__lede">
            Cette version a été compilée sans configuration de comptes. Elle ne
            peut pas vous identifier, et ne peut donc rien ouvrir.
          </p>
          <p class="porte__note">
            Téléchargez à nouveau Derovia depuis le site officiel.
          </p>
        </div>
      </div>`;
    return;
  }

  racine.innerHTML = `
    <div class="launcher__body">
      <div class="porte">
        <div class="launcher__mark" aria-hidden="true">D</div>
        <h1 class="porte__titre" id="porte-titre">${textes.titre}</h1>
        <p class="porte__lede" id="porte-lede">${textes.lede}</p>

        <div class="segmented" role="tablist" id="porte-switch">
          <div class="segmented__thumb" id="porte-thumb"></div>
          <button type="button" role="tab" data-mode="signin"
                  aria-selected="${mode === "signin"}">Se connecter</button>
          <button type="button" role="tab" data-mode="signup"
                  aria-selected="${mode === "signup"}">Créer un compte</button>
        </div>

        <form class="auth-form porte__form" id="porte-form">
          <label class="field" id="porte-nom-champ"${mode === "signup" ? "" : " hidden"}>
            <span class="field__label">Nom</span>
            <span class="input">
              <input type="text" id="porte-nom" autocomplete="name" placeholder="Votre nom" />
            </span>
          </label>

          <label class="field">
            <span class="field__label">Adresse e-mail</span>
            <span class="input">
              <input type="email" id="porte-email" autocomplete="email"
                     placeholder="vous@exemple.fr" required />
            </span>
          </label>

          <label class="field">
            <span class="field__label">Mot de passe</span>
            <span class="input">
              <input type="password" id="porte-motdepasse"
                     autocomplete="${mode === "signup" ? "new-password" : "current-password"}"
                     placeholder="••••••••" required minlength="6" />
            </span>
          </label>

          <p class="auth-form__note" id="porte-message" hidden></p>

          <button class="btn btn--primary btn--block" type="submit" id="porte-action">
            ${textes.action}
          </button>
        </form>

        <p class="porte__note">
          Vos fichiers ne quittent jamais cette machine : le compte ne sert
          qu'à vous identifier et à retrouver vos scénarios.
        </p>
      </div>
    </div>`;

  racine.querySelector("#porte-switch")?.addEventListener("click", (evenement) => {
    const choisi = (evenement.target as HTMLElement | null)?.closest<HTMLElement>("[data-mode]")
      ?.dataset["mode"];
    if (choisi !== "signin" && choisi !== "signup") return;
    if (choisi === mode) return;
    mode = choisi;
    render(racine, onConnecte);
  });

  racine.querySelector<HTMLFormElement>("#porte-form")?.addEventListener("submit", (evenement) => {
    evenement.preventDefault();
    void soumettre(racine, onConnecte);
  });

  racine.querySelector<HTMLInputElement>("#porte-email")?.focus();
  positionnerPastille(racine);
}

/** Fait glisser la pastille du selecteur sous l'onglet actif. */
function positionnerPastille(racine: HTMLElement): void {
  const pastille = racine.querySelector<HTMLElement>("#porte-thumb");
  const actif = racine.querySelector<HTMLElement>('#porte-switch button[aria-selected="true"]');
  if (!pastille || !actif) return;
  pastille.style.width = `${actif.offsetWidth}px`;
  pastille.style.transform = `translateX(${actif.offsetLeft - 2}px)`;
}

/** Affiche un message sous le formulaire. */
function message(racine: HTMLElement, texte: string, ton: "negative" | "positive"): void {
  const zone = racine.querySelector<HTMLElement>("#porte-message");
  if (!zone) return;
  zone.textContent = texte;
  zone.dataset["tone"] = ton;
  zone.hidden = false;
}

/** Envoie le formulaire et rend compte du resultat. */
async function soumettre(racine: HTMLElement, onConnecte: () => void): Promise<void> {
  const action = racine.querySelector<HTMLButtonElement>("#porte-action");
  const email = racine.querySelector<HTMLInputElement>("#porte-email")?.value.trim() ?? "";
  const motDePasse = racine.querySelector<HTMLInputElement>("#porte-motdepasse")?.value ?? "";
  const nom = racine.querySelector<HTMLInputElement>("#porte-nom")?.value.trim();
  if (!action) return;

  const libelle = TEXTES[mode].action;
  action.disabled = true;
  action.textContent = TEXTES[mode].occupe;
  const zone = racine.querySelector<HTMLElement>("#porte-message");
  if (zone) zone.hidden = true;

  const resultat =
    mode === "signup"
      ? await inscrire(email, motDePasse, nom)
      : await connecter(email, motDePasse);

  action.disabled = false;
  action.textContent = libelle;

  switch (resultat.etat) {
    case "connecte":
      onConnecte();
      break;
    case "confirmation_requise":
      message(
        racine,
        `Compte créé. Ouvrez le lien envoyé à ${resultat.email} pour l'activer, puis revenez vous connecter.`,
        "positive",
      );
      break;
    case "erreur":
      message(racine, resultat.message, "negative");
      break;
  }
}

/**
 * Montre la porte, et n'appelle `onConnecte` qu'une fois un compte etabli.
 *
 * Si une session est deja ouverte — le cas ordinaire, d'un lancement a
 * l'autre — la porte ne s'affiche meme pas.
 */
export async function exigerCompte(racine: HTMLElement, onConnecte: () => void): Promise<void> {
  let entre = false;
  const entrer = (): void => {
    if (entre) return;
    entre = true;
    racine.hidden = true;
    racine.innerHTML = "";
    onConnecte();
  };

  // Une session peut s'ouvrir sans passer par ce formulaire : renouvellement
  // au demarrage, ou validation depuis un autre onglet.
  surChangementDeCompte((compte: Compte | null) => {
    if (compte) entrer();
  });

  const existant = await compteActuel().catch(() => null);
  if (existant) {
    entrer();
    return;
  }

  racine.hidden = false;
  render(racine, entrer);
}
