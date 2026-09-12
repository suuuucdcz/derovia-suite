/**
 * Point d'entree de la suite Derovia.
 *
 * L'application ouvre sur l'ecran de lancement de la suite. L'espace de travail
 * de l'outil selectionne est demarre a la demande.
 */

import "./styles.css";

import { mountLauncher } from "./launcher";
import { preparer } from "./setup";
import type { SuiteTool } from "./suite";
import { startWorkspace as startArbitrage } from "./workspace";
import { startConvertWorkspace } from "./workspace-convert";
import { startCompressWorkspace } from "./workspace-compress";

function routeTool(tool: SuiteTool): void {
  if (tool.id === "arbitrage") {
    void startArbitrage();
  } else if (tool.id === "convertisseur") {
    startConvertWorkspace();
  } else if (tool.id === "compresseur") {
    startCompressWorkspace();
  }
}

const ecranPreparation = document.querySelector<HTMLElement>("#setup");
const ecranLanceur = document.querySelector<HTMLElement>("#launcher");

/** Passe de la preparation a l'accueil de la suite. */
function entrerDansLaSuite(): void {
  if (ecranPreparation) ecranPreparation.hidden = true;
  if (ecranLanceur) ecranLanceur.hidden = false;
  mountLauncher(routeTool);
}

if (ecranPreparation) {
  void preparer(ecranPreparation, entrerDansLaSuite);
} else {
  entrerDansLaSuite();
}
