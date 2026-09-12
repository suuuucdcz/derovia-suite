/**
 * Les scenarios d'arbitrage enregistres.
 *
 * C'est ce que la fenetre de connexion promet : « Retrouvez vos scenarios sur
 * tous vos postes ». Les hypotheses voyagent en JSON, exactement dans la forme
 * que le moteur Rust attend — aucune traduction intermediaire a maintenir.
 *
 * La securite ne repose pas sur ce module : les politiques RLS du projet font
 * qu'un compte ne peut lire ni modifier que ses propres lignes, quoi que le
 * client demande.
 */

import { supabase } from "./auth";
import type { Arbitrage } from "./types";

/** Un scenario tel qu'il revient du serveur. */
export interface ScenarioEnregistre {
  /** L'identifiant de la ligne. */
  id: string;
  /** Le nom donne par l'utilisateur. */
  nom: string;
  /** La situation de depart : immobilier, vehicule ou materiel. */
  preset: string;
  /** Les hypotheses completes. */
  hypotheses: Arbitrage;
  /** La date de derniere modification, au format ISO. */
  modifieLe: string;
}

/** La forme des lignes renvoyees par la table. */
interface Ligne {
  id: string;
  nom: string;
  preset: string;
  hypotheses: Arbitrage;
  modifie_le: string;
}

/** Erreur lisible quand le serveur n'est pas joignable. */
function indisponible(): Error {
  return new Error("La synchronisation n'est pas configurée.");
}

/** Traduit une ligne en scenario. */
function versScenario(ligne: Ligne): ScenarioEnregistre {
  return {
    id: ligne.id,
    nom: ligne.nom,
    preset: ligne.preset,
    hypotheses: ligne.hypotheses,
    modifieLe: ligne.modifie_le,
  };
}

/**
 * Les scenarios du compte connecte, du plus recent au plus ancien.
 *
 * Aucun filtre sur l'utilisateur n'est ecrit ici : le RLS s'en charge, et le
 * dupliquer cote client donnerait l'illusion que c'est lui qui protege.
 */
export async function listerScenarios(): Promise<ScenarioEnregistre[]> {
  const client = supabase();
  if (!client) throw indisponible();

  const { data, error } = await client
    .from("scenarios")
    .select("id, nom, preset, hypotheses, modifie_le")
    .order("modifie_le", { ascending: false });

  if (error) throw new Error(error.message);
  return (data as Ligne[]).map(versScenario);
}

/** Enregistre un nouveau scenario. */
export async function enregistrerScenario(
  nom: string,
  preset: string,
  hypotheses: Arbitrage,
): Promise<ScenarioEnregistre> {
  const client = supabase();
  if (!client) throw indisponible();

  const { data: session } = await client.auth.getUser();
  const utilisateur = session.user?.id;
  if (!utilisateur) throw new Error("Connectez-vous pour enregistrer un scénario.");

  const { data, error } = await client
    .from("scenarios")
    .insert({ utilisateur, nom, preset, hypotheses })
    .select("id, nom, preset, hypotheses, modifie_le")
    .single();

  if (error) throw new Error(error.message);
  return versScenario(data as Ligne);
}

/** Met a jour les hypotheses d'un scenario existant. */
export async function majScenario(id: string, hypotheses: Arbitrage): Promise<void> {
  const client = supabase();
  if (!client) throw indisponible();

  const { error } = await client.from("scenarios").update({ hypotheses }).eq("id", id);
  if (error) throw new Error(error.message);
}

/** Supprime un scenario. */
export async function supprimerScenario(id: string): Promise<void> {
  const client = supabase();
  if (!client) throw indisponible();

  const { error } = await client.from("scenarios").delete().eq("id", id);
  if (error) throw new Error(error.message);
}
