/**
 * Point d'entree de la suite Derovia.
 *
 * L'application ouvre sur l'ecran de lancement de la suite. L'espace de travail
 * de l'outil selectionne est demarre a la demande.
 */

import "./styles.css";

import { mountLauncher } from "./launcher";
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

mountLauncher((tool) => {
  routeTool(tool);
});
