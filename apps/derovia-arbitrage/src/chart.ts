/**
 * Le graphique de trajectoire, en SVG genere a la main.
 *
 * Aucune bibliotheque de graphiques : deux courbes et une grille ne justifient
 * pas 200 ko de dependance, et un SVG ecrit ici hérite directement des couleurs
 * du design system, donc du theme clair ou sombre, sans configuration.
 */

import { formatCompactEur } from "./format";
import type { YearPoint } from "./types";

const WIDTH = 720;
const HEIGHT = 300;
const MARGIN = { top: 18, right: 22, bottom: 30, left: 64 };
const INNER_WIDTH = WIDTH - MARGIN.left - MARGIN.right;
const INNER_HEIGHT = HEIGHT - MARGIN.top - MARGIN.bottom;

/**
 * Choisit un pas de graduation qui tombe rond.
 *
 * Un axe gradue tous les 37 412 € est illisible ; on force le pas sur 1, 2 ou 5
 * fois une puissance de dix, ce qui donne toujours des reperes que l'oeil situe.
 */
function niceStep(range: number, targetTicks: number): number {
  if (range <= 0) return 1;
  const rough = range / targetTicks;
  const magnitude = 10 ** Math.floor(Math.log10(rough));
  const normalized = rough / magnitude;
  const factor = normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10;
  return factor * magnitude;
}

function escapeText(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** Construit le chemin d'une courbe. */
function linePath(points: Array<[number, number]>): string {
  return points.map(([x, y], index) => `${index === 0 ? "M" : "L"}${x} ${y}`).join(" ");
}

/**
 * Construit la bande comprise entre les deux courbes.
 *
 * Remplir separement l'aire sous chaque courbe donne deux translucides
 * superposes, donc du gris sale. Une seule bande, teintee de la couleur de
 * celui qui mene, montre exactement ce qu'on cherche a lire : l'ecart.
 */
function bandPath(upper: Array<[number, number]>, lower: Array<[number, number]>): string {
  if (upper.length === 0 || lower.length === 0) return "";
  const back = [...lower].reverse().map(([x, y]) => `L${x} ${y}`).join(" ");
  return `${linePath(upper)} ${back} Z`;
}

/**
 * Rend la trajectoire des deux patrimoines nets.
 *
 * Renvoie le balisage SVG complet, pret a etre insere. Toutes les valeurs
 * tracees sont numeriques et les seuls textes proviennent du moteur, echappes
 * par prudence.
 */
export function renderTimelineChart(
  timeline: YearPoint[],
  breakEvenYear: number | null,
  labels: { buy: string; rent: string },
): string {
  const first = timeline[0];
  const last = timeline[timeline.length - 1];
  if (!first || !last || timeline.length < 2) {
    return `<svg class="chart" viewBox="0 0 ${WIDTH} ${HEIGHT}" role="img"></svg>`;
  }

  const values = timeline.flatMap((point) => [point.buyNetWorth, point.rentNetWorth]);
  const rawMin = Math.min(...values, 0);
  const rawMax = Math.max(...values);
  const step = niceStep(rawMax - rawMin, 4);
  const min = Math.floor(rawMin / step) * step;
  const max = Math.ceil(rawMax / step) * step;
  const span = max - min || 1;

  const yearSpan = last.year - first.year || 1;
  const x = (year: number) => MARGIN.left + ((year - first.year) / yearSpan) * INNER_WIDTH;
  const y = (value: number) => MARGIN.top + ((max - value) / span) * INNER_HEIGHT;

  const buyPoints = timeline.map((p): [number, number] => [x(p.year), y(p.buyNetWorth)]);
  const rentPoints = timeline.map((p): [number, number] => [x(p.year), y(p.rentNetWorth)]);
  const leader = last.buyNetWorth >= last.rentNetWorth ? "buy" : "rent";

  // Graduations horizontales et leurs etiquettes.
  const gridlines: string[] = [];
  for (let value = min; value <= max + step / 2; value += step) {
    const yy = y(value);
    gridlines.push(
      `<line class="chart__grid" x1="${MARGIN.left}" y1="${yy}" x2="${WIDTH - MARGIN.right}" y2="${yy}" />`,
      `<text class="chart__axis" x="${MARGIN.left - 10}" y="${yy + 4}" text-anchor="end">${escapeText(formatCompactEur(value))}</text>`,
    );
  }

  // Graduations d'annees : environ six reperes, quel que soit l'horizon.
  const yearStep = Math.max(1, Math.round(yearSpan / 6));
  const yearTicks: string[] = [];
  for (let year = first.year; year <= last.year; year += yearStep) {
    yearTicks.push(
      `<text class="chart__axis" x="${x(year)}" y="${HEIGHT - 8}" text-anchor="middle">${year}</text>`,
    );
  }
  if ((last.year - first.year) % yearStep !== 0) {
    yearTicks.push(
      `<text class="chart__axis" x="${x(last.year)}" y="${HEIGHT - 8}" text-anchor="middle">${last.year}</text>`,
    );
  }

  // Le point de bascule : la seule annotation qui merite d'etre sur le trace.
  let marker = "";
  if (breakEvenYear !== null && breakEvenYear >= first.year && breakEvenYear <= last.year) {
    const mx = x(breakEvenYear);
    const anchor = mx > MARGIN.left + INNER_WIDTH * 0.72 ? "end" : "start";
    const offset = anchor === "end" ? -8 : 8;
    marker =
      `<line class="chart__marker" x1="${mx}" y1="${MARGIN.top}" x2="${mx}" y2="${MARGIN.top + INNER_HEIGHT}" />` +
      `<text class="chart__marker-label" x="${mx + offset}" y="${MARGIN.top + 12}" text-anchor="${anchor}">Bascule · année ${breakEvenYear}</text>`;
  }

  const description = `Patrimoine net comparé de l'année ${first.year} à l'année ${last.year} : ${escapeText(labels.buy)} contre ${escapeText(labels.rent)}.`;

  return `<svg class="chart" viewBox="0 0 ${WIDTH} ${HEIGHT}" role="img" aria-label="${description}" preserveAspectRatio="xMidYMid meet">
  ${gridlines.join("\n  ")}
  <path class="chart__band" data-tone="${leader}" d="${bandPath(buyPoints, rentPoints)}" />
  ${marker}
  <path class="chart__line chart__line--rent" d="${linePath(rentPoints)}" />
  <path class="chart__line chart__line--buy" d="${linePath(buyPoints)}" />
  ${yearTicks.join("\n  ")}
</svg>`;
}
