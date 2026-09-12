/**
 * L'authentification de la suite, adossee a Supabase.
 *
 * La cle `anon` utilisee ici est publique par conception : elle est embarquee
 * dans tout binaire distribue, et ce sont les regles RLS du projet qui
 * protegent les donnees — pas le secret de cette cle. La cle `service_role`,
 * elle, ne doit jamais approcher un client.
 *
 * Le module reste utilisable meme si la configuration est absente : dans ce
 * cas [`authDisponible`] repond faux et l'interface propose de continuer sans
 * compte, plutot que d'afficher un formulaire qui echouerait a chaque essai.
 */

import { createClient, type Session, type SupabaseClient } from "@supabase/supabase-js";

const URL_PROJET = import.meta.env.VITE_SUPABASE_URL?.trim();
const CLE_ANON = import.meta.env.VITE_SUPABASE_ANON_KEY?.trim();

/** Le client, cree une seule fois — ou `null` si la configuration manque. */
const client: SupabaseClient | null =
  URL_PROJET && CLE_ANON
    ? createClient(URL_PROJET, CLE_ANON, {
        auth: {
          // La session survit a la fermeture de la fenetre : WebView2 conserve
          // le stockage local entre deux lancements.
          persistSession: true,
          autoRefreshToken: true,
          // Aucune redirection a intercepter : l'application n'est pas servie
          // depuis une URL ou Supabase pourrait renvoyer un fragment.
          detectSessionInUrl: false,
        },
      })
    : null;

/** Ce que l'interface a besoin de savoir d'un compte connecte. */
export interface Compte {
  /** L'identifiant Supabase de l'utilisateur. */
  id: string;
  /** Son adresse e-mail. */
  email: string;
}

/** L'issue d'une tentative d'inscription ou de connexion. */
export type ResultatAuth =
  | { etat: "connecte"; compte: Compte }
  | { etat: "confirmation_requise"; email: string }
  | { etat: "erreur"; message: string };

/** Vrai quand la configuration Supabase est presente. */
export function authDisponible(): boolean {
  return client !== null;
}

/** Traduit une session Supabase en compte, si elle en porte un. */
function compteDepuis(session: Session | null): Compte | null {
  const utilisateur = session?.user;
  if (!utilisateur?.email) return null;
  return { id: utilisateur.id, email: utilisateur.email };
}

/**
 * Traduit les messages de Supabase, qui sont en anglais.
 *
 * Laisser passer « Invalid login credentials » dans une interface francaise
 * serait negligent ; les cas non prevus sont transmis tels quels plutot que
 * remplaces par un message vague.
 */
function messageLisible(message: string): string {
  const connus: Record<string, string> = {
    "Invalid login credentials": "Adresse e-mail ou mot de passe incorrect.",
    "Email not confirmed":
      "Ce compte n'est pas encore confirmé. Ouvrez le lien reçu par e-mail.",
    "User already registered": "Un compte existe déjà avec cette adresse.",
    "Password should be at least 6 characters":
      "Le mot de passe doit faire au moins 6 caractères.",
    "Unable to validate email address: invalid format": "Cette adresse e-mail n'est pas valide.",
    "Signups not allowed for this instance":
      "Les inscriptions sont désactivées sur ce projet.",
  };
  return connus[message] ?? message;
}

/** Le compte actuellement connecte, s'il y en a un. */
export async function compteActuel(): Promise<Compte | null> {
  if (!client) return null;
  const { data } = await client.auth.getSession();
  return compteDepuis(data.session);
}

/**
 * Cree un compte.
 *
 * `nom` accompagne l'inscription dans les metadonnees du compte : c'est lui que
 * le declencheur Supabase reprend pour remplir le profil. Sans cela, le champ
 * « Nom » du formulaire serait saisi puis perdu.
 */
export async function inscrire(
  email: string,
  motDePasse: string,
  nom?: string,
): Promise<ResultatAuth> {
  if (!client) return { etat: "erreur", message: "L'authentification n'est pas configurée." };

  const { data, error } = await client.auth.signUp({
    email,
    password: motDePasse,
    ...(nom?.trim() ? { options: { data: { nom: nom.trim() } } } : {}),
  });
  if (error) return { etat: "erreur", message: messageLisible(error.message) };

  const compte = compteDepuis(data.session);
  if (compte) return { etat: "connecte", compte };

  // Sans session, c'est que le projet exige une confirmation par e-mail.
  // Ce n'est pas un echec : il faut le dire, pas le masquer.
  return { etat: "confirmation_requise", email };
}

/** Ouvre une session. */
export async function connecter(email: string, motDePasse: string): Promise<ResultatAuth> {
  if (!client) return { etat: "erreur", message: "L'authentification n'est pas configurée." };

  const { data, error } = await client.auth.signInWithPassword({
    email,
    password: motDePasse,
  });
  if (error) return { etat: "erreur", message: messageLisible(error.message) };

  const compte = compteDepuis(data.session);
  return compte
    ? { etat: "connecte", compte }
    : { etat: "erreur", message: "La connexion n'a pas abouti." };
}

/** Ferme la session. */
export async function deconnecter(): Promise<void> {
  await client?.auth.signOut();
}

/**
 * Previent a chaque changement d'etat de connexion.
 *
 * Supabase rafraichit les jetons tout seul et peut donc invalider une session
 * sans que l'utilisateur ait rien fait : l'interface doit suivre.
 */
export function surChangementDeCompte(rappel: (compte: Compte | null) => void): () => void {
  if (!client) return () => {};
  const { data } = client.auth.onAuthStateChange((_evenement, session) => {
    rappel(compteDepuis(session));
  });
  return () => data.subscription.unsubscribe();
}
