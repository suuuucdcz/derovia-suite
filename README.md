# Derovia

Suite d'outils d'automatisation pour Windows. Chaque outil est une application
de bureau autonome ; tous partagent un socle commun — le même design system, les
mêmes primitives de calcul, la même manière de présenter un résultat.

**Outils disponibles :**
- **Derovia Arbitrage** — acheter ou louer, chiffres à l'appui.
- **Derovia Convertisseur** — conversion de documents et d'images (.docx, .pdf, Markdown, HTML, texte, PNG/JPEG/WebP).
- **Derovia Compresseur** — optimisation et compression de n'importe quel fichier (images, PDF, archives).

---

## Démarrer

```bash
npm install
npm run dev
```

> **Sous PowerShell**, `npm` peut échouer avec *« l'exécution de scripts est
> désactivée sur ce système »*. Windows bloque alors `npm.ps1`, pas le projet.
> Deux issues : appeler `npm.cmd run dev`, qui n'est pas un script PowerShell,
> ou autoriser une fois pour toutes les scripts locaux de ton compte avec
> `Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned`.

La fenêtre s'ouvre sur le lanceur de la suite et permet de naviguer instantanément
entre les outils. Les modifications du frontend sont rechargées à chaud ; celles
du Rust relancent l'application.

Pour produire l'installeur Windows (`.msi` et `.exe`) :

```bash
npm run build
```

Les paquets sortent dans `target/release/bundle/`.

## Vérifier

```bash
powershell -ExecutionPolicy Bypass -File scripts/verifier.ps1
```

Format, clippy sans indulgence, tests Rust et vérification TypeScript, en une
commande. C'est ce qui doit passer avant tout commit.

---

## Organisation

```
derovia/
├─ crates/
│  ├─ derovia-core/         Primitives partagées : taux, prêts, monnaie, erreurs, format_bytes
│  ├─ derovia-arbitrage/    Le moteur « acheter ou louer » et ses trois presets
│  ├─ derovia-convert/      Le moteur de conversion documentaire (.docx, .pdf, markdown, HTML, images)
│  ├─ derovia-compress/     Le moteur d'optimisation et compression universel (images, PDF, zip)
│  └─ derovia-engines/      Installation et pilotage des moteurs externes (Pandoc)
├─ packages/
│  └─ derovia-design/       Le design system : jetons, socle, composants, dropzone
├─ apps/
│  └─ derovia-arbitrage/    L'application Tauri (frontend TypeScript + src-tauri)
├─ docs/                    Architecture, modèle de calcul, design
└─ scripts/                 Vérification et documentation hors-ligne
```

La règle qui tient l'ensemble : **aucune règle de calcul dans le frontend.**
Le TypeScript saisit des hypothèses et affiche un résultat ; tout le reste vit
dans les crates Rust, où c'est testable sans ouvrir de fenêtre.

Deuxième règle : **un outil n'invente pas de composant visuel.** Il assemble
ceux de `@derovia/design`. Quand il en manque un, sa place est dans le design
system, pas dans la feuille de style de l'outil — sinon la suite se disloque au
troisième outil.

## Pile technique

| Choix | Pourquoi |
|---|---|
| Rust + Tauri 2 | Un `.exe` natif de quelques mégaoctets, sans runtime à installer, et le calcul là où il est vérifiable |
| MSVC (`stable-x86_64-pc-windows-msvc`) | La seule cible officiellement supportée par Tauri sur Windows |
| TypeScript strict + Vite | Zéro framework : l'interface est légère et rien ne casse à la montée de version |
| Aucune bibliothèque de graphiques | Le SVG est généré à la main, hérite du thème et pèse zéro |

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — comment les morceaux s'emboîtent
- [Le modèle de calcul](docs/MOTEUR.md) — ce que le moteur compare, et pourquoi
- [Le design system](docs/DESIGN.md) — jetons, composants, règles d'usage
- Documentation Rust hors-ligne : `powershell -File scripts/docs.ps1`

## Licence

Projet privé. Tous droits réservés.
