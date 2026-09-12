/**
 * Point d'entree de la suite Derovia.
 *
 * Trois etapes, dans cet ordre : la porte, qui n'ouvre qu'a un compte ; la
 * preparation, qui recupere les moteurs de conversion au premier lancement ;
 * puis l'accueil de la suite. L'espace de travail d'un outil est demarre a la
 * demande.
 */

import "./styles.css";

import { mountLauncher } from "./launcher";
import { exigerCompte } from "./porte";
import { surveillerMiseAJour } from "./maj";
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

const ecranPorte = document.querySelector<HTMLElement>("#porte");
const ecranPreparation = document.querySelector<HTMLElement>("#setup");
const ecranLanceur = document.querySelector<HTMLElement>("#launcher");

/** Passe de la preparation a l'accueil de la suite. */
function entrerDansLaSuite(): void {
  if (ecranPreparation) ecranPreparation.hidden = true;
  if (ecranLanceur) ecranLanceur.hidden = false;
  mountLauncher(routeTool);
}

/** Une fois le compte etabli, installe les moteurs manquants puis ouvre. */
function apresConnexion(): void {
  if (ecranPreparation) {
    void preparer(ecranPreparation, entrerDansLaSuite);
  } else {
    entrerDansLaSuite();
  }
}

if (ecranPorte) {
  void exigerCompte(ecranPorte, apresConnexion);
} else {
  apresConnexion();
}

// La verification part apres l'affichage : elle ne doit jamais retarder
// l'ouverture de la fenetre, ni l'empecher si le reseau est coupe.
window.setTimeout(() => void surveillerMiseAJour(), 3000);
