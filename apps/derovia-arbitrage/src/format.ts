/**
 * Mise en forme des nombres, cote interface.
 *
 * Le moteur Rust formate deja pour ses propres sorties texte ; l'interface a
 * besoin des memes regles pour les etiquettes d'axes et les champs, d'ou ce
 * pendant en TypeScript. `Intl` applique la typographie francaise sans qu'on
 * ait a la reimplementer.
 */

const EUR = new Intl.NumberFormat("fr-FR", {
  style: "currency",
  currency: "EUR",
  maximumFractionDigits: 0,
});

const EUR_PRECISE = new Intl.NumberFormat("fr-FR", {
  style: "currency",
  currency: "EUR",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

const PLAIN = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 2 });

/** Un montant arrondi a l'euro : `1 234 568 €`. */
export function formatEur(value: number): string {
  return Number.isFinite(value) ? EUR.format(value) : "—";
}

/** Un montant au centime : `1 234,56 €`. Reserve aux mensualites. */
export function formatEurPrecise(value: number): string {
  return Number.isFinite(value) ? EUR_PRECISE.format(value) : "—";
}

/**
 * Un montant abrege pour les axes : `420 k€`, `1,2 M€`.
 *
 * Un axe de graphique n'a pas la place d'afficher sept chiffres, et personne
 * ne lit l'unite d'un patrimoine de plusieurs centaines de milliers d'euros.
 */
export function formatCompactEur(value: number): string {
  if (!Number.isFinite(value)) return "—";
  const magnitude = Math.abs(value);
  if (magnitude >= 1_000_000) return `${PLAIN.format(value / 1_000_000)} M€`;
  if (magnitude >= 1_000) return `${PLAIN.format(Math.round(value / 1_000))} k€`;
  return `${PLAIN.format(Math.round(value))} €`;
}

/** Un ecart signe : `+21 250 €` ou `−9 480 €`. */
export function formatSignedEur(value: number): string {
  if (!Number.isFinite(value)) return "—";
  const sign = value >= 0 ? "+" : "−";
  return `${sign}${EUR.format(Math.abs(value))}`;
}

/** Un taux stocke en decimale, affiche en pourcentage : `3,5 %`. */
export function formatRate(rate: number): string {
  return `${PLAIN.format(rate * 100)} %`;
}

/** Un pourcentage deja exprime comme tel : `4,05 %`. */
export function formatPercent(percent: number): string {
  return `${PLAIN.format(percent)} %`;
}

/** Un nombre d'annees, avec son pluriel. */
export function formatYears(years: number): string {
  return years <= 1 ? `${years} an` : `${years} ans`;
}

/** Formate une taille en octets : `14,50 Mo`. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0\u{202f}o";
  const KIB = 1024;
  const MIB = KIB * 1024;
  const GIB = MIB * 1024;

  if (bytes < KIB) return `${bytes}\u{202f}o`;
  if (bytes < MIB) return `${PLAIN.format(Math.round((bytes / KIB) * 10) / 10)}\u{202f}Ko`;
  if (bytes < GIB) return `${PLAIN.format(Math.round((bytes / MIB) * 100) / 100)}\u{202f}Mo`;
  return `${PLAIN.format(Math.round((bytes / GIB) * 100) / 100)}\u{202f}Go`;
}
