/**
 * Derovia Arbitrage — l'espace de travail de l'outil.
 *
 * Le frontend ne calcule rien. Il tient trois choses : les hypotheses saisies,
 * leur mise en forme, et l'appel au moteur a chaque modification. Le formulaire
 * n'est reconstruit qu'au changement de situation ; une frappe ne redessine que
 * la colonne de resultats, sinon le champ perdrait le focus a chaque caractere.
 *
 * Rien ne demarre au chargement du module : c'est le lanceur qui appelle
 * [`startWorkspace`] quand l'utilisateur ouvre l'outil.
 */

import { analyser, catalogue } from "./api";
import { compteActuel } from "./auth";
import { escapeHtml, setWorkspaceChrome } from "./dom";
import {
  enregistrerScenario,
  listerScenarios,
  supprimerScenario,
  type ScenarioEnregistre,
} from "./scenarios";
import { renderTimelineChart } from "./chart";
import { formatEur, formatEurPrecise, formatPercent, formatSignedEur, formatYears } from "./format";
import type { Analysis, Arbitrage, PresetCard } from "./types";

// --- Description des champs -------------------------------------------------

interface FieldSpec {
  key: string;
  label: string;
  unit: string;
  step: number;
  min: number;
  max: number;
  hint?: string;
  read: (scenario: Arbitrage) => number;
  write: (scenario: Arbitrage, value: number) => void;
}

interface FieldGroup {
  title: string;
  collapsed: boolean;
  fields: FieldSpec[];
  available?: (scenario: Arbitrage) => boolean;
}

type Read = (scenario: Arbitrage) => number;
type Write = (scenario: Arbitrage, value: number) => void;

function money(key: string, label: string, read: Read, write: Write, hint?: string): FieldSpec {
  return { key, label, unit: "€", step: 100, min: 0, max: 1e9, read, write, ...(hint && { hint }) };
}

/** Un champ de taux. Affiche en pourcentage, stocke en decimale. */
function rate(
  key: string,
  label: string,
  read: Read,
  write: Write,
  options: { min?: number; hint?: string } = {},
): FieldSpec {
  return {
    key,
    label,
    unit: "%",
    step: 0.1,
    min: options.min ?? 0,
    max: 100,
    read: (scenario) => Math.round(read(scenario) * 10_000) / 100,
    write: (scenario, value) => write(scenario, value / 100),
    ...(options.hint && { hint: options.hint }),
  };
}

function duration(key: string, label: string, read: Read, write: Write): FieldSpec {
  return { key, label, unit: "ans", step: 1, min: 0, max: 40, read, write };
}

const GROUPS: FieldGroup[] = [
  {
    title: "Le bien",
    collapsed: false,
    fields: [
      money("price", "Prix d'achat", (s) => s.buy.price, (s, v) => (s.buy.price = v)),
      rate(
        "acquisitionFees",
        "Frais d'acquisition",
        (s) => s.buy.acquisitionFees,
        (s, v) => (s.buy.acquisitionFees = v),
        { hint: "notaire, carte grise, installation" },
      ),
      money("downPayment", "Apport", (s) => s.buy.downPayment, (s, v) => (s.buy.downPayment = v)),
    ],
  },
  {
    title: "Le financement",
    collapsed: false,
    fields: [
      rate("loanRate", "Taux du crédit", (s) => s.buy.loanRate, (s, v) => (s.buy.loanRate = v)),
      duration("loanYears", "Durée", (s) => s.buy.loanYears, (s, v) => (s.buy.loanYears = v)),
      rate(
        "loanInsurance",
        "Assurance emprunteur",
        (s) => s.buy.loanInsurance,
        (s, v) => (s.buy.loanInsurance = v),
      ),
    ],
  },
  {
    title: "La location",
    collapsed: false,
    fields: [
      money("monthlyRent", "Loyer mensuel", (s) => s.rent.monthlyRent, (s, v) => (s.rent.monthlyRent = v)),
      money(
        "rentCharges",
        "Charges mensuelles",
        (s) => s.rent.monthlyCharges,
        (s, v) => (s.rent.monthlyCharges = v),
      ),
      rate(
        "rentIndexation",
        "Indexation annuelle",
        (s) => s.rent.rentIndexation,
        (s, v) => (s.rent.rentIndexation = v),
      ),
      money("entryFees", "Frais d'entrée", (s) => s.rent.entryFees, (s, v) => (s.rent.entryFees = v)),
      money("deposit", "Dépôt de garantie", (s) => s.rent.deposit, (s, v) => (s.rent.deposit = v)),
    ],
  },
  {
    title: "Charges et revente",
    collapsed: true,
    fields: [
      rate(
        "maintenance",
        "Entretien annuel",
        (s) => s.buy.maintenance,
        (s, v) => (s.buy.maintenance = v),
        { hint: "en % du prix" },
      ),
      money(
        "yearlyCharges",
        "Charges annuelles",
        (s) => s.buy.yearlyCharges,
        (s, v) => (s.buy.yearlyCharges = v),
        "taxe foncière, assurance, copropriété",
      ),
      rate(
        "valueChange",
        "Évolution de la valeur",
        (s) => s.buy.valueChange,
        (s, v) => (s.buy.valueChange = v),
        { min: -50, hint: "négatif = décote" },
      ),
      rate(
        "firstYearDrop",
        "Décote immédiate",
        (s) => s.buy.firstYearDrop,
        (s, v) => (s.buy.firstYearDrop = v),
        { hint: "subie dès l'achat" },
      ),
      rate("resaleFees", "Frais de revente", (s) => s.buy.resaleFees, (s, v) => (s.buy.resaleFees = v)),
    ],
  },
  {
    title: "Le marché",
    collapsed: true,
    fields: [
      rate(
        "investmentReturn",
        "Rendement du placement",
        (s) => s.market.investmentReturn,
        (s, v) => (s.market.investmentReturn = v),
        { hint: "ce que rapporte l'argent non immobilisé" },
      ),
      rate(
        "inflation",
        "Inflation",
        (s) => s.market.inflation,
        (s, v) => (s.market.inflation = v),
        { hint: "appliquée aux charges" },
      ),
    ],
  },
  {
    title: "Fiscalité",
    collapsed: true,
    available: (scenario) => scenario.tax !== null,
    fields: [
      rate("vatRate", "TVA", (s) => s.tax?.vatRate ?? 0, (s, v) => s.tax && (s.tax.vatRate = v), {
        hint: "récupérable",
      }),
      rate(
        "profitTax",
        "Impôt sur les bénéfices",
        (s) => s.tax?.profitTax ?? 0,
        (s, v) => s.tax && (s.tax.profitTax = v),
      ),
      duration(
        "depreciationYears",
        "Amortissement",
        (s) => s.tax?.depreciationYears ?? 0,
        (s, v) => s.tax && (s.tax.depreciationYears = v),
      ),
    ],
  },
];

// --- Etat -------------------------------------------------------------------

let presets: PresetCard[] = [];
let active: PresetCard | null = null;
let scenario: Arbitrage | null = null;
let pending: number | undefined;
let latest: Analysis | null = null;

let formColumn: HTMLElement | null = null;
let resultColumn: HTMLElement | null = null;
let subtitle: HTMLElement | null = null;
let started = false;

// --- Aides ------------------------------------------------------------------

function isEngineError(value: unknown): value is { kind: "invalid"; field: string; reason: string } {
  return typeof value === "object" && value !== null && "reason" in value && "field" in value;
}

function visibleGroups(current: Arbitrage): FieldGroup[] {
  return GROUPS.filter((group) => group.available?.(current) ?? true);
}

// --- Construction du formulaire ---------------------------------------------

function fieldMarkup(spec: FieldSpec, current: Arbitrage): string {
  const value = spec.read(current);
  const hint = spec.hint ? `<span class="field__hint">${escapeHtml(spec.hint)}</span>` : "";
  return `<label class="field">
    <span class="field__label">${escapeHtml(spec.label)}${hint}</span>
    <span class="input">
      <input type="number" data-field="${spec.key}" value="${value}"
             step="${spec.step}" min="${spec.min}" max="${spec.max}" inputmode="decimal" />
      <span class="input__unit">${escapeHtml(spec.unit)}</span>
    </span>
  </label>`;
}

function groupMarkup(group: FieldGroup, current: Arbitrage): string {
  const fields = `<div class="field-grid">${group.fields.map((f) => fieldMarkup(f, current)).join("")}</div>`;
  if (!group.collapsed) {
    return `<div class="field-block">
      <h3 class="field-block__title">${escapeHtml(group.title)}</h3>
      ${fields}
    </div>`;
  }
  return `<details class="disclosure">
    <summary>${escapeHtml(group.title)}</summary>
    <div class="field-block__body">${fields}</div>
  </details>`;
}

function renderForm(): void {
  if (!formColumn || !active || !scenario) return;
  const current = scenario;

  const segments = presets
    .map(
      (preset, index) =>
        `<button type="button" role="tab" data-preset="${escapeHtml(preset.id)}"
           aria-selected="${preset.id === active?.id}" data-index="${index}">${escapeHtml(preset.title)}</button>`,
    )
    .join("");

  formColumn.innerHTML = `
    <section class="card">
      <div class="card__header"><h2 class="card__title">Situation</h2></div>
      <div class="segmented" role="tablist" id="preset-switch">
        <div class="segmented__thumb" id="preset-thumb"></div>
        ${segments}
      </div>
      <label class="field horizon">
        <span class="field__label">Horizon
          <span class="field__value" id="horizon-readout">${formatYears(current.horizonYears)}</span>
        </span>
        <input class="slider" type="range" id="horizon" min="1" max="40" value="${current.horizonYears}" />
      </label>
    </section>

    <section class="card">
      <div class="card__header"><h2 class="card__title">Hypothèses</h2></div>
      ${visibleGroups(current).map((group) => groupMarkup(group, current)).join("")}
    </section>
  `;

  positionThumb();
}

/** Fait glisser la pastille du controle segmente sous l'onglet actif. */
function positionThumb(): void {
  const thumb = document.querySelector<HTMLElement>("#preset-thumb");
  const selected = document.querySelector<HTMLElement>('#preset-switch button[aria-selected="true"]');
  if (!thumb || !selected) return;
  thumb.style.width = `${selected.offsetWidth}px`;
  thumb.style.transform = `translateX(${selected.offsetLeft - 2}px)`;
}

// --- Rendu des resultats ----------------------------------------------------

function verdictReading(analysis: Analysis, preset: PresetCard): string {
  const { verdict, indifferenceReturn } = analysis;
  const horizon = formatYears(verdict.horizonYears);
  const winner = verdict.recommendation === "buy" ? preset.buyLabel : preset.rentLabel;
  const gap = formatEur(Math.abs(verdict.netAdvantage));

  let sentence: string;
  if (verdict.recommendation === "tooClose") {
    sentence = `Sur ${horizon}, les deux options se valent : ${gap} d'écart, soit moins que la marge d'erreur des hypothèses. Le choix se joue ailleurs que sur l'argent.`;
  } else {
    // Le libelle garde sa casse : « LOA / LLD » ne se met pas en minuscules.
    sentence = `Sur ${horizon}, ${winner} laisse ${gap} de patrimoine en plus.`;
  }

  if (verdict.breakEvenYear !== null) {
    sentence += ` L'achat repasse devant à partir de l'année ${verdict.breakEvenYear}.`;
  } else if (verdict.recommendation === "rent") {
    sentence += ` L'achat ne rattrape jamais son retard sur cet horizon.`;
  }

  if (indifferenceReturn !== null) {
    sentence += ` La conclusion bascule si le capital placé rapporte plus de ${formatPercent(indifferenceReturn)} par an.`;
  }
  return sentence;
}

function ledgerRow(label: string, buy: string, rent: string): string {
  return `<tr>
    <td>${escapeHtml(label)}</td>
    <td data-tone="buy">${buy}</td>
    <td data-tone="rent">${rent}</td>
  </tr>`;
}

function renderResults(analysis: Analysis, preset: PresetCard): void {
  if (!resultColumn) return;
  const { verdict } = analysis;
  const tone =
    verdict.recommendation === "buy" ? "buy" : verdict.recommendation === "rent" ? "rent" : "neutral";
  const headline =
    verdict.recommendation === "tooClose"
      ? "Trop serré pour trancher"
      : verdict.recommendation === "buy"
        ? preset.buyLabel
        : preset.rentLabel;

  // L'ecart est signe du point de vue de l'achat. A cote du nom du gagnant, un
  // nombre negatif se lirait a l'envers : on affiche donc ce que le gagnant
  // rapporte en plus, toujours positif.
  const delta =
    verdict.recommendation === "tooClose"
      ? `${formatEur(Math.abs(verdict.netAdvantage))} d'écart`
      : formatSignedEur(Math.abs(verdict.netAdvantage));

  const facts: string[] = [];
  if (verdict.breakEvenYear !== null) {
    facts.push(`<span class="pill">Bascule <strong>année ${verdict.breakEvenYear}</strong></span>`);
  }
  if (analysis.indifferenceReturn !== null) {
    facts.push(
      `<span class="pill">Seuil de rendement <strong>${formatPercent(analysis.indifferenceReturn)}</strong></span>`,
    );
  }
  facts.push(
    `<span class="pill">Effort mensuel <strong>${formatEurPrecise(verdict.buy.monthlyEffortFirstYear)}</strong> contre <strong>${formatEurPrecise(verdict.rent.monthlyEffortFirstYear)}</strong></span>`,
  );

  const hasTax = verdict.buy.taxSaved > 0 || verdict.rent.taxSaved > 0;

  resultColumn.innerHTML = `
    <section class="verdict rise">
      <div class="verdict__eyebrow">Sur ${escapeHtml(formatYears(verdict.horizonYears))}</div>
      <div class="verdict__headline" data-tone="${tone}">
        ${escapeHtml(headline)}
        <span class="verdict__delta tabular">${escapeHtml(delta)}</span>
      </div>
      <p class="verdict__reading">${escapeHtml(verdictReading(analysis, preset))}</p>
      <div class="verdict__facts">${facts.join("")}</div>
    </section>

    <section class="card">
      <div class="card__header">
        <h2 class="card__title">Patrimoine net</h2>
        <span class="card__hint">placement, bien et dette cumulés</span>
      </div>
      ${renderTimelineChart(verdict.timeline, verdict.breakEvenYear, { buy: preset.buyLabel, rent: preset.rentLabel })}
      <div class="legend">
        <span class="legend__item"><span class="legend__swatch" data-tone="buy"></span>${escapeHtml(preset.buyLabel)}</span>
        <span class="legend__item"><span class="legend__swatch" data-tone="rent"></span>${escapeHtml(preset.rentLabel)}</span>
      </div>
    </section>

    <section class="card card--flush">
      <table class="ledger">
        <thead>
          <tr>
            <th>À l'horizon</th>
            <th>${escapeHtml(preset.buyLabel)}</th>
            <th>${escapeHtml(preset.rentLabel)}</th>
          </tr>
        </thead>
        <tbody>
          ${ledgerRow("Sorties de trésorerie", formatEur(verdict.buy.totalOutflow), formatEur(verdict.rent.totalOutflow))}
          ${ledgerRow("Capital placé", formatEur(verdict.buy.portfolio), formatEur(verdict.rent.portfolio))}
          ${ledgerRow("Valeur du bien, nette", formatEur(verdict.buy.assetValue), formatEur(verdict.rent.assetValue))}
          ${ledgerRow("Dette résiduelle", formatEur(verdict.buy.remainingDebt), formatEur(verdict.rent.remainingDebt))}
          ${ledgerRow("Intérêts payés", formatEur(verdict.buy.interestPaid), formatEur(verdict.rent.interestPaid))}
          ${hasTax ? ledgerRow("Économie d'impôt", formatEur(verdict.buy.taxSaved), formatEur(verdict.rent.taxSaved)) : ""}
        </tbody>
        <tfoot>
          <tr>
            <td>Patrimoine net</td>
            <td data-tone="buy">${formatEur(verdict.buy.netWorth)}</td>
            <td data-tone="rent">${formatEur(verdict.rent.netWorth)}</td>
          </tr>
        </tfoot>
      </table>
    </section>
  `;
}

function renderError(message: string): void {
  if (!resultColumn) return;
  resultColumn.innerHTML = `<div class="notice">${escapeHtml(message)}</div>`;
}

// --- Boucle de calcul -------------------------------------------------------

async function recompute(): Promise<void> {
  if (!scenario || !active) return;
  const preset = active;
  try {
    const analysis = await analyser(scenario, preset.id);
    latest = analysis;
    renderResults(analysis, preset);
  } catch (error) {
    latest = null;
    renderError(
      isEngineError(error)
        ? `${error.field} : ${error.reason}`
        : "Le moteur de calcul n'a pas répondu.",
    );
  }
}

function scheduleRecompute(): void {
  window.clearTimeout(pending);
  pending = window.setTimeout(() => void recompute(), 120);
}

function selectPreset(id: string): void {
  const preset = presets.find((candidate) => candidate.id === id);
  if (!preset) return;
  active = preset;
  scenario = structuredClone(preset.scenario);
  if (subtitle) subtitle.textContent = preset.subtitle;
  renderForm();
  void recompute();
}

// --- Ecoute des interactions ------------------------------------------------

function wire(): void {
  formColumn?.addEventListener("input", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement) || !scenario) return;

    if (target.id === "horizon") {
      scenario.horizonYears = Number(target.value);
      const readout = document.querySelector<HTMLElement>("#horizon-readout");
      if (readout) readout.textContent = formatYears(scenario.horizonYears);
      scheduleRecompute();
      return;
    }

    const key = target.dataset["field"];
    if (!key) return;
    const spec = GROUPS.flatMap((group) => group.fields).find((field) => field.key === key);
    if (!spec) return;

    const value = Number(target.value);
    if (!Number.isFinite(value)) return;
    spec.write(scenario, value);
    scheduleRecompute();
  });

  formColumn?.addEventListener("click", (event) => {
    const button = (event.target as HTMLElement | null)?.closest<HTMLElement>("[data-preset]");
    const id = button?.dataset["preset"];
    if (id) selectPreset(id);
  });

  window.addEventListener("resize", positionThumb);

  document.querySelector("#action-reset")?.addEventListener("click", () => {
    if (active) selectPreset(active.id);
  });

  document.querySelector("#action-copy")?.addEventListener("click", () => {
    void copySummary();
  });
  document.querySelector("#action-scenarios")?.addEventListener("click", () => {
    void ouvrirScenarios();
  });
}

async function copySummary(): Promise<void> {
  const button = document.querySelector<HTMLButtonElement>("#action-copy");
  if (!latest || !active || !button) return;
  const { verdict } = latest;
  const lines = [
    `Derovia Arbitrage — ${active.title}`,
    `Horizon : ${formatYears(verdict.horizonYears)}`,
    ``,
    `${active.buyLabel} : ${formatEur(verdict.buy.netWorth)} de patrimoine net`,
    `${active.rentLabel} : ${formatEur(verdict.rent.netWorth)} de patrimoine net`,
    `Écart : ${formatSignedEur(verdict.netAdvantage)}`,
    verdict.breakEvenYear !== null
      ? `Bascule : année ${verdict.breakEvenYear}`
      : `Bascule : jamais sur cet horizon`,
    ``,
    verdictReading(latest, active),
  ];

  const original = button.textContent ?? "Copier le résumé";
  try {
    await navigator.clipboard.writeText(lines.join("\n"));
    button.textContent = "Copié";
  } catch {
    button.textContent = "Copie impossible";
  }
  window.setTimeout(() => (button.textContent = original), 1600);
}

// --- Demarrage --------------------------------------------------------------

/**
 * Ouvre l'outil dans la coquille applicative.
 *
 * Appelable plusieurs fois sans dommage : revenir au lanceur puis rouvrir
 * l'outil ne doit pas rebrancher deux fois les ecouteurs.
 */
export async function startWorkspace(): Promise<void> {
  // La barre de titre est reinstallee a chaque ouverture : un autre outil a pu
  // la remplacer entre-temps, et ses boutons n'existent plus.
  setWorkspaceChrome({
    title: "Arbitrage",
    subtitle: "Acheter ou louer, chiffres à l'appui",
    actions: `
      <button class="btn btn--secondary" type="button" id="action-scenarios">Mes scénarios</button>
      <button class="btn btn--secondary" type="button" id="action-reset">Réinitialiser</button>
      <button class="btn btn--primary" type="button" id="action-copy">Copier le résumé</button>`,
  });

  formColumn = document.querySelector<HTMLElement>("#form-column");
  resultColumn = document.querySelector<HTMLElement>("#result-column");
  subtitle = document.querySelector<HTMLElement>("#app-subtitle");

  // Les boutons viennent d'etre recrees : on les rebranche a chaque fois.
  document.querySelector("#action-reset")?.addEventListener("click", () => {
    if (active) selectPreset(active.id);
  });
  document.querySelector("#action-copy")?.addEventListener("click", () => {
    void copySummary();
  });

  if (presets.length === 0) {
    try {
      presets = await catalogue();
    } catch {
      renderError("Impossible de charger les situations proposées.");
      return;
    }
  }

  const currentPreset = active ?? presets[0];
  if (!currentPreset) {
    renderError("Aucune situation disponible.");
    return;
  }

  if (!started) {
    started = true;
    wire();
  }

  selectPreset(currentPreset.id);
}


// --- Scenarios enregistres ---------------------------------------------------

/** Affiche un message dans la fenetre des scenarios. */
function messageScenarios(texte: string, ton: "negative" | "positive"): void {
  const zone = document.querySelector<HTMLElement>("#scenarios-message");
  if (!zone) return;
  zone.textContent = texte;
  zone.dataset["tone"] = ton;
  zone.hidden = false;
}

/** Efface le message de la fenetre des scenarios. */
function effacerMessageScenarios(): void {
  const zone = document.querySelector<HTMLElement>("#scenarios-message");
  if (!zone) return;
  zone.hidden = true;
  zone.textContent = "";
}

/** Dessine la liste des scenarios enregistres. */
function rendreListeScenarios(liste: ScenarioEnregistre[]): void {
  const conteneur = document.querySelector<HTMLElement>("#scenarios-liste");
  if (!conteneur) return;

  if (liste.length === 0) {
    conteneur.innerHTML =
      '<p class="scenario-list__vide">Aucun scénario enregistré pour l\'instant.</p>';
    return;
  }

  conteneur.innerHTML = liste
    .map((enregistre) => {
      const date = new Date(enregistre.modifieLe).toLocaleDateString("fr-FR", {
        day: "numeric",
        month: "long",
        year: "numeric",
      });
      const horizon = formatYears(enregistre.hypotheses.horizonYears);
      return [
        '<div class="scenario-item">',
        '  <div class="scenario-item__texte">',
        `    <div class="scenario-item__nom">${escapeHtml(enregistre.nom)}</div>`,
        '    <div class="scenario-item__detail">',
        `      ${escapeHtml(enregistre.preset)} · ${escapeHtml(horizon)} · ${escapeHtml(date)}`,
        "    </div>",
        "  </div>",
        `  <button class="btn btn--secondary btn--sm" type="button" data-charger="${escapeHtml(enregistre.id)}">Ouvrir</button>`,
        `  <button class="file-item__remove" type="button" data-supprimer="${escapeHtml(enregistre.id)}" title="Supprimer">✕</button>`,
        "</div>",
      ].join("\n");
    })
    .join("");

  for (const bouton of conteneur.querySelectorAll<HTMLElement>("[data-charger]")) {
    bouton.addEventListener("click", () => {
      const cible = liste.find((candidat) => candidat.id === bouton.dataset["charger"]);
      if (cible) chargerScenario(cible);
    });
  }

  for (const bouton of conteneur.querySelectorAll<HTMLElement>("[data-supprimer]")) {
    bouton.addEventListener("click", () => {
      const id = bouton.dataset["supprimer"];
      if (id) void retirerScenario(id);
    });
  }
}

/** Reprend un scenario enregistre dans l'outil. */
function chargerScenario(enregistre: ScenarioEnregistre): void {
  const preset = presets.find((candidat) => candidat.id === enregistre.preset);
  if (preset) active = preset;
  scenario = structuredClone(enregistre.hypotheses);

  if (subtitle && active) subtitle.textContent = active.subtitle;
  renderForm();
  void recompute();
  fermerScenarios();
}

/** Supprime un scenario, puis rafraichit la liste. */
async function retirerScenario(id: string): Promise<void> {
  try {
    await supprimerScenario(id);
    rendreListeScenarios(await listerScenarios());
    effacerMessageScenarios();
  } catch (erreur: unknown) {
    messageScenarios(erreur instanceof Error ? erreur.message : String(erreur), "negative");
  }
}

/** Ferme la fenetre des scenarios. */
function fermerScenarios(): void {
  const fenetre = document.querySelector<HTMLElement>("#scenarios-modal");
  if (fenetre) fenetre.hidden = true;
}

/** Vrai une fois les ecouteurs de la fenetre branches. */
let scenariosBranches = false;

/**
 * Ouvre la fenetre des scenarios.
 *
 * Sans compte connecte, elle explique pourquoi plutot que d'afficher une liste
 * vide qui laisserait croire a une perte de donnees.
 */
async function ouvrirScenarios(): Promise<void> {
  const fenetre = document.querySelector<HTMLElement>("#scenarios-modal");
  if (!fenetre) return;

  if (!scenariosBranches) {
    scenariosBranches = true;
    for (const cible of document.querySelectorAll<HTMLElement>("[data-close-scenarios]")) {
      cible.addEventListener("click", fermerScenarios);
    }
    document.addEventListener("keydown", (evenement) => {
      if (evenement.key === "Escape" && !fenetre.hidden) fermerScenarios();
    });
    document
      .querySelector<HTMLFormElement>("#scenario-save")
      ?.addEventListener("submit", (evenement) => {
        evenement.preventDefault();
        void sauvegarderScenarioCourant();
      });
  }

  fenetre.hidden = false;
  effacerMessageScenarios();

  const champ = document.querySelector<HTMLInputElement>("#scenario-nom");
  if (champ && active && scenario) {
    champ.value = `${active.title} — ${formatYears(scenario.horizonYears)}`;
  }

  const compte = await compteActuel();
  if (!compte) {
    rendreListeScenarios([]);
    messageScenarios(
      "Connectez-vous pour enregistrer vos scénarios et les retrouver sur vos autres postes.",
      "negative",
    );
    return;
  }

  try {
    rendreListeScenarios(await listerScenarios());
  } catch (erreur: unknown) {
    messageScenarios(erreur instanceof Error ? erreur.message : String(erreur), "negative");
  }
}

/** Enregistre les hypotheses actuelles sous le nom saisi. */
async function sauvegarderScenarioCourant(): Promise<void> {
  const bouton = document.querySelector<HTMLButtonElement>("#scenario-enregistrer");
  const champ = document.querySelector<HTMLInputElement>("#scenario-nom");
  const nom = champ?.value.trim();

  if (!scenario || !active) return;
  if (!nom) {
    messageScenarios("Donnez un nom à ce scénario.", "negative");
    return;
  }

  if (bouton) {
    bouton.disabled = true;
    bouton.textContent = "Enregistrement…";
  }

  try {
    await enregistrerScenario(nom, active.id, scenario);
    messageScenarios(`« ${nom} » est enregistré.`, "positive");
    rendreListeScenarios(await listerScenarios());
  } catch (erreur: unknown) {
    messageScenarios(erreur instanceof Error ? erreur.message : String(erreur), "negative");
  } finally {
    if (bouton) {
      bouton.disabled = false;
      bouton.textContent = "Enregistrer";
    }
  }
}
