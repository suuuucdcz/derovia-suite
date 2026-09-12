/**
 * Le catalogue de la suite Derovia.
 *
 * Un seul endroit decrit les outils : l'ecran de lancement et la barre laterale
 * s'en servent tous les deux. Ajouter un outil a la suite, c'est ajouter une
 * ligne ici — pas retoucher deux balisages qui finiraient par diverger.
 */

/** Un outil de la suite, disponible ou annonce. */
export interface SuiteTool {
  /** Identifiant stable, utilise dans les attributs de donnees. */
  id: string;
  /** Le nom affiche. */
  name: string;
  /** Le glyphe de la carte et de la barre laterale. */
  glyph: string;
  /** Ce que l'outil fait, en une phrase. */
  description: string;
  /** Faux tant que l'outil n'est pas livre : la carte est alors inerte. */
  available: boolean;
}

/** Les outils de la suite, dans l'ordre d'affichage. */
export const SUITE_TOOLS: SuiteTool[] = [
  {
    id: "arbitrage",
    name: "Arbitrage",
    glyph: "⇄",
    description: "Acheter ou louer : la réponse chiffrée, sur l'horizon qui vous concerne.",
    available: true,
  },
  {
    id: "convertisseur",
    name: "Convertisseur",
    glyph: "⎘",
    description: "Word, PDF, images, Markdown, HTML, texte : convertir de l'un vers l'autre.",
    available: true,
  },
  {
    id: "compresseur",
    name: "Compresseur",
    glyph: "⇲",
    description: "Réduire le poids de n'importe quel fichier : images, PDF, documents et archives.",
    available: true,
  },
  {
    id: "prevision",
    name: "Prévision",
    glyph: "◷",
    description: "Projeter une trésorerie et voir venir les creux avant qu'ils arrivent.",
    available: false,
  },
  {
    id: "devis",
    name: "Devis",
    glyph: "◫",
    description: "Chiffrer une prestation et produire le document, sans passer par un tableur.",
    available: false,
  },
  {
    id: "veille",
    name: "Veille",
    glyph: "◎",
    description: "Suivre un marché et être alerté quand une hypothèse de départ bouge.",
    available: false,
  },
];

/** La version affichee dans le pied de l'ecran de lancement. */
export const SUITE_VERSION = "0.1.0";
