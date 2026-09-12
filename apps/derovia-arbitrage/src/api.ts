/**
 * Le pont vers le moteur Rust.
 *
 * En production, tout passe par `invoke` : aucune regle de calcul ne vit dans
 * le frontend, qui ne fait que saisir des hypotheses et afficher un resultat.
 *
 * Hors de la fenetre Tauri — c'est-a-dire quand on ouvre le serveur Vite dans
 * un navigateur pour travailler la mise en page — le moteur n'existe pas. On
 * sert alors un enregistrement de vraies sorties du moteur, fige dans
 * `dev-fixtures.json`. Ce n'est pas une reimplementation du calcul : les
 * chiffres ne bougent pas quand on modifie un champ, et la console le dit.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  Analysis,
  Arbitrage,
  CompressOptions,
  CompressResult,
  ConversionResult,
  ConvertOptions,
  EngineError,
  EngineStatus,
  FichierSauvegarde,
  PresetCard,
} from "./types";

interface DevFixtures {
  catalogue: PresetCard[];
  analyses: Record<string, Analysis>;
}

let fixtures: Promise<DevFixtures> | null = null;
let warned = false;

/** Vrai quand le code tourne bien dans la fenetre Tauri. */
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function devFixtures(): Promise<DevFixtures> {
  if (!warned) {
    warned = true;
    console.warn(
      "[Derovia] Moteur Rust indisponible : affichage d'un enregistrement fige. " +
        "Lancer `npm run app` pour la vraie application.",
    );
  }
  fixtures ??= import("./dev-fixtures.json").then((module) => module.default as DevFixtures);
  return fixtures;
}

/** Les trois situations proposees au demarrage. */
export async function catalogue(): Promise<PresetCard[]> {
  if (inTauri()) return invoke<PresetCard[]>("catalogue");
  return (await devFixtures()).catalogue;
}

/**
 * Lance un arbitrage.
 *
 * Rejette avec une `EngineError` quand les hypotheses sont inexploitables ; son
 * champ `reason` est deja redige pour l'utilisateur.
 */
export async function analyser(scenario: Arbitrage, presetId: string): Promise<Analysis> {
  if (inTauri()) return invoke<Analysis>("analyser", { scenario });
  const recorded = (await devFixtures()).analyses[presetId];
  if (!recorded) throw { kind: "invalid", field: "Moteur", reason: "aucun enregistrement" };
  return recorded;
}

/**
 * Refus commun aux operations de fichier hors de la fenetre Tauri.
 *
 * Le moteur vit dans le binaire Rust. Dans un simple navigateur il n'existe
 * pas, et il n'y a rien d'honnete a simuler : renvoyer le fichier d'entree
 * sous un autre nom, ou annoncer un taux de compression invente, donnerait
 * l'illusion que l'outil fonctionne.
 */
function moteurIndisponible(operation: string): EngineError {
  return {
    kind: "failure",
    operation,
    message:
      "Cette opération a besoin du moteur de l'application. Lancez Derovia plutôt que le serveur de développement.",
  };
}

/** Convertit un document vers un format cible. */
export async function convertirDocument(
  fileName: string,
  inputBytes: number[],
  targetFormat: string,
  options: ConvertOptions = {},
): Promise<ConversionResult> {
  if (!inTauri()) throw moteurIndisponible("conversion");
  return invoke<ConversionResult>("convertir_document", {
    fileName,
    inputBytes,
    targetFormat,
    options,
  });
}

/** Compresse et optimise un fichier. */
export async function compresserFichier(
  fileName: string,
  inputBytes: number[],
  options: CompressOptions = { level: "balanced" },
): Promise<CompressResult> {
  if (!inTauri()) throw moteurIndisponible("compression");
  return invoke<CompressResult>("compresser_fichier", { fileName, inputBytes, options });
}

/** Enregistre un fichier sur le disque dans le dossier Téléchargements. */
export async function sauvegarderFichier(
  fileName: string,
  data: number[],
): Promise<FichierSauvegarde> {
  if (!inTauri()) throw moteurIndisponible("sauvegarde");
  return invoke<FichierSauvegarde>("sauvegarder_fichier", { fileName, data });
}

/** Ouvre l'explorateur Windows et sélectionne le fichier. */
export async function ouvrirDansExplorateur(chemin: string): Promise<void> {
  if (!inTauri()) throw moteurIndisponible("explorateur");
  return invoke<void>("ouvrir_dans_explorateur", { chemin });
}

/**
 * Rend lisible une erreur remontee par le moteur.
 *
 * `CoreError` a deux formes : une validation d'entree (`field` / `reason`) et
 * un echec de traitement (`operation` / `message`). Les deux doivent aboutir au
 * meme endroit, sinon la moitie des messages se perd derriere un libelle generique.
 */
export function messageDErreur(erreur: unknown): string {
  if (typeof erreur === "object" && erreur !== null) {
    if ("reason" in erreur) return String((erreur as { reason: unknown }).reason);
    if ("message" in erreur) return String((erreur as { message: unknown }).message);
  }
  if (typeof erreur === "string") return erreur;
  return "Le moteur n'a pas répondu.";
}

/**
 * Taille maximale d'un fichier accepte par les outils de fichiers.
 *
 * Les octets traversent le pont Tauri sous forme de tableau JSON de nombres :
 * chaque octet y coute plusieurs caracteres. Un fichier de 200 Mo produirait un
 * tableau de 200 millions d'elements et figerait la fenetre avant meme
 * d'atteindre le moteur. La limite est donc celle du transport, pas celle du
 * moteur Rust — qui, lui, traiterait bien plus.
 */
export const TAILLE_MAX_OCTETS = 48 * 1024 * 1024;

/**
 * Lit un fichier depose et le prepare pour le pont Tauri.
 *
 * Rejette avec une `EngineError` lisible si le fichier depasse ce que le
 * transport peut encaisser, plutot que de laisser l'interface se bloquer.
 */
export async function lireFichier(file: File): Promise<number[]> {
  if (file.size > TAILLE_MAX_OCTETS) {
    const limite = Math.round(TAILLE_MAX_OCTETS / (1024 * 1024));
    throw {
      kind: "failure",
      operation: "lecture",
      message: `Ce fichier dépasse ${limite} Mo, la taille maximale acceptée pour l'instant.`,
    } satisfies EngineError;
  }
  return Array.from(new Uint8Array(await file.arrayBuffer()));
}

/** L'etat du moteur Pandoc. */
export async function moteurStatut(): Promise<EngineStatus> {
  if (!inTauri()) throw moteurIndisponible("moteur");
  return invoke<EngineStatus>("moteur_statut");
}

/** Telecharge et installe Pandoc. */
export async function installerMoteur(): Promise<EngineStatus> {
  if (!inTauri()) throw moteurIndisponible("moteur");
  return invoke<EngineStatus>("installer_moteur");
}

/**
 * Suit l'avancement du telechargement du moteur.
 *
 * Renvoie la fonction a appeler pour cesser d'ecouter.
 */
export async function suivreInstallation(
  onProgress: (recus: number, attendus: number) => void,
): Promise<() => void> {
  if (!inTauri()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<[number, number]>("moteur://progression", (evenement) => {
    const [recus, attendus] = evenement.payload;
    onProgress(recus, attendus);
  });
}
