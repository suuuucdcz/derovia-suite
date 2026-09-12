/**
 * Le miroir TypeScript des types du moteur Rust.
 *
 * Ces interfaces doivent rester alignees sur `derovia-arbitrage::model` et
 * `::result`. Les champs Rust sont serialises en camelCase, d'ou les noms
 * ci-dessous. Un desaccord se voit immediatement : `analyser` renvoie une
 * erreur de deserialisation plutot qu'un resultat silencieusement faux.
 */

/** Un taux annuel sous forme decimale : `0.035` vaut 3,5 %. */
export type Rate = number;

/** Les hypotheses de marche, communes aux deux options. */
export interface Market {
  investmentReturn: Rate;
  inflation: Rate;
}

/** L'option « acheter ». */
export interface Buy {
  price: number;
  acquisitionFees: Rate;
  downPayment: number;
  loanRate: Rate;
  loanYears: number;
  loanInsurance: Rate;
  maintenance: Rate;
  yearlyCharges: number;
  valueChange: Rate;
  firstYearDrop: Rate;
  resaleFees: Rate;
}

/** L'option « louer ». */
export interface Rent {
  monthlyRent: number;
  rentIndexation: Rate;
  monthlyCharges: number;
  entryFees: number;
  deposit: number;
  buyoutOption: number | null;
}

/** La fiscalite d'un bien professionnel. */
export interface Tax {
  vatRecoverable: boolean;
  vatRate: Rate;
  profitTax: Rate;
  depreciationYears: number;
}

/** Un arbitrage complet, tel qu'envoye au moteur. */
export interface Arbitrage {
  horizonYears: number;
  market: Market;
  buy: Buy;
  rent: Rent;
  tax: Tax | null;
}

/** Le bilan d'une des deux options a l'horizon. */
export interface Outcome {
  totalOutflow: number;
  netWorth: number;
  assetValue: number;
  remainingDebt: number;
  portfolio: number;
  monthlyEffortFirstYear: number;
  interestPaid: number;
  taxSaved: number;
}

/** Un point annuel de la trajectoire. */
export interface YearPoint {
  year: number;
  buyNetWorth: number;
  rentNetWorth: number;
  buyCumulativeCost: number;
  rentCumulativeCost: number;
  assetValue: number;
  remainingDebt: number;
}

/** La decision rendue par le moteur. */
export type Recommendation = "buy" | "rent" | "tooClose";

/** Le resultat complet d'un arbitrage. */
export interface Verdict {
  recommendation: Recommendation;
  horizonYears: number;
  breakEvenYear: number | null;
  buy: Outcome;
  rent: Outcome;
  netAdvantage: number;
  timeline: YearPoint[];
}

/** Le verdict, accompagne du seuil qui le renverserait. */
export interface Analysis {
  verdict: Verdict;
  /** En pourcentage, ou `null` si aucun rendement plausible ne fait basculer. */
  indifferenceReturn: number | null;
}

/** Une situation proposee au demarrage, hypotheses incluses. */
export interface PresetCard {
  id: string;
  title: string;
  subtitle: string;
  buyLabel: string;
  rentLabel: string;
  scenario: Arbitrage;
}

/** L'erreur renvoyee par le moteur. */
export type EngineError =
  | { kind: "invalid"; field: string; reason: string }
  | { kind: "failure"; operation: string; message: string };

/** Options de conversion documentaire. */
export interface ConvertOptions {
  pageSize?: string | undefined;
  marginMm?: number | undefined;
  imageQuality?: number | undefined;
}

/** Resultat d'une conversion de document. */
export interface ConversionResult {
  fileName: string;
  outputFormat: string;
  data: number[];
  originalSize: number;
  outputSize: number;
}

/** Profil de compression. */
export type CompressionLevel = "lossless" | "balanced" | "maximum";

/** Options de compression. */
export interface CompressOptions {
  level: CompressionLevel;
  quality?: number | undefined;
  resizePercent?: number | undefined;
}

/** Resultat d'une compression de fichier. */
export interface CompressResult {
  fileName: string;
  originalSize: number;
  compressedSize: number;
  savedBytes: number;
  reductionPercent: number;
  data: number[];
}

/** Informations sur un fichier sauvegardé sur le disque. */
export interface FichierSauvegarde {
  nom: string;
  chemin: string;
  taille: number;
}


/** L'etat d'installation d'un moteur de conversion externe. */
export interface EngineStatus {
  /** L'identifiant court du moteur. */
  id: string;
  /** Son nom lisible. */
  label: string;
  installed: boolean;
  version: string | null;
  /** Poids du telechargement, en octets. */
  downloadBytes: number;
  /** Poids sur le disque une fois installe, en octets. */
  installedBytes: number;
  /** Vrai quand le moteur ne s'installe qu'a la demande expresse. */
  optional: boolean;
}

/** L'avancement du telechargement d'un moteur. */
export interface ProgressionMoteur {
  id: string;
  recus: number;
  attendus: number;
}
