# Architecture

## Le principe

Derovia est une suite, pas une collection d'applications. Ce qui en fait une
suite tient en deux couches partagées : les **crates** (le calcul) et le
**design system** (l'apparence). Un nouvel outil apporte son moteur métier et
son interface ; il ne réinvente ni l'un ni l'autre socle.

```
                     ┌──────────────────────────┐
   apps/             │  derovia-arbitrage       │   ← une app = un produit
                     │  (Tauri : TS + src-tauri)│
                     └────────┬────────┬────────┘
                              │        │
        packages/  ┌──────────▼──┐  ┌──▼─────────────────┐  crates/
                   │ @derovia/   │  │ derovia-arbitrage  │  ← moteur métier
                   │ design      │  └──┬─────────────────┘
                   └─────────────┘     │
                                    ┌──▼──────────────┐
                                    │ derovia-core    │  ← primitives suite
                                    └─────────────────┘
```

## Les couches

### `crates/derovia-core`

Les primitives que tout outil de la suite réutilise : le type `Rate` (frontière
entre les pourcentages de l'interface et les décimales du calcul), les prêts
amortissables simulés au mois, la mise en forme monétaire française, le type
d'erreur commun.

Il ne connaît aucun produit en particulier. C'est ce qui garantit que deux
outils différents ne calculeront jamais une mensualité de deux façons.

### `crates/derovia-arbitrage`

Le moteur « acheter ou louer » : le modèle d'entrée, la simulation, les trois
presets, le verdict. Voir [MOTEUR.md](MOTEUR.md).

### `crates/derovia-convert`

Le moteur de conversion documentaire : lecture OpenXML DOCX, extraction de texte
PDF, analyse HTML, génération PDF vectorielle, Markdown, HTML, texte et images.

Une limite est assumée et signalée à l'utilisateur plutôt que contournée par un
résultat approximatif : le `.doc` binaire de Word 97-2003 est refusé avec la
consigne de réenregistrer en `.docx`. Le lire correctement demande un analyseur
de conteneur OLE ; ce qui existait en balayait les octets et perdait tous les
accents.

Le WebP est lu **et** écrit, via l'encodeur libwebp activé sur le crate `image`.
Il travaille sans perte.
Zéro dépendance externe, 100% Rust hors-ligne.

### `crates/derovia-compress`

Le moteur de compression : optimisation d'images (JPEG, PNG, redimensionnement),
recompression des flux et élagage des objets orphelins d'un PDF (`lopdf`), mise
en archive ZIP pour tout le reste.

Deux garanties valent pour tous les formats : le fichier rendu n'est **jamais**
plus lourd que l'original — faute de mieux, l'original est restitué et le bilan
annonce 0 % — et le nom de sortie décrit toujours le contenu réellement produit,
un PNG réencodé en JPEG ressortant en `.jpg`.

### `crates/derovia-engines`

Installation et pilotage des moteurs de conversion externes. Aujourd'hui un
seul : **Pandoc**, la référence du domaine pour le balisage.

Il n'est pas embarqué dans l'installeur — 40 Mo à télécharger, 223 Mo sur le
disque — mais récupéré à la demande. La version **et son empreinte SHA-256**
sont épinglées dans le code : une archive qui ne correspond pas est rejetée et
rien n'est installé. Télécharger « la dernière version » rendrait l'installation
ni reproductible ni vérifiable.

La dégradation est volontaire : **sans Pandoc, tout continue de fonctionner**
via le moteur interne. Pandoc améliore la fidélité — tableaux, notes, styles —
il ne déverrouille rien.

Deux limites lui sont propres, et le moteur interne garde donc ces cas : Pandoc
**ne lit pas le PDF**, et **ne produit un PDF qu'avec un moteur LaTeX** installé
à côté, que la suite n'embarque pas.

### `packages/derovia-design`

Cinq feuilles CSS, chargées dans cet ordre : les jetons, le socle, les
composants (avec dropzones et cartes de fichiers), l'écran de lancement. Un outil importe `@derovia/design` et ne
redéfinit que sa couleur d'accent.

### `apps/derovia-arbitrage`

L'application. `src/` est le frontend TypeScript, `src-tauri/` le binaire Rust
qui ouvre la fenêtre et expose deux commandes : `catalogue` et `analyser`.

Le frontend se lit en six morceaux :

| Module | Rôle |
|---|---|
| `main.ts` | Point d'entrée et routeur d'outils de la suite |
| `suite.ts` | Le catalogue des outils de la suite |
| `launcher.ts` | L'écran de lancement, synchronisation de la barre latérale et fenêtre de compte |
| `workspace.ts` | L'outil Arbitrage |
| `workspace-convert.ts` | L'outil Convertisseur de documents |
| `workspace-compress.ts` | L'outil Compresseur et optimiseur universel |

L'espace de travail n'est démarré que lorsque l'utilisateur ouvre l'outil :
tant qu'il reste sur l'accueil, le moteur n'est pas sollicité.

## Les frontières

**Le frontend ne calcule rien.** Il saisit des hypothèses, appelle le moteur,
met en forme le résultat. Toute tentative de « juste calculer ça côté JS » finit
par diverger du moteur, silencieusement.

**Les types traversent la frontière tels quels.** `src/types.ts` est le miroir
exact de `model.rs` et `result.rs`, sérialisés en camelCase. Un désaccord se
manifeste par une erreur de désérialisation, pas par un résultat faux.

**Le binaire de l'app ne contient pas de logique métier.** `main.rs` fait
transiter les appels vers `derovia-arbitrage`, et rien d'autre. C'est ce qui
permet de tester tout le comportement sans ouvrir de fenêtre.

## Le mode développement du frontend

Ouvrir le serveur Vite dans un navigateur plutôt que dans la fenêtre Tauri est
utile pour travailler la mise en page. Le moteur Rust n'existe alors pas ;
`src/api.ts` sert un enregistrement de **vraies sorties du moteur**
(`dev-fixtures.json`), figé, et le signale dans la console.

Ce n'est pas une réimplémentation du calcul en TypeScript — les chiffres ne
bougent pas quand on modifie un champ. On régénère l'enregistrement avec :

```bash
cargo run -p derovia-arbitrage --example fixtures > apps/derovia-arbitrage/src/dev-fixtures.json
```

## Discipline de qualité

Les lints sont déclarés une fois dans le `Cargo.toml` du workspace et hérités
par chaque crate. `unsafe_code` est **interdit**, pas seulement découragé.
`unwrap`, `expect`, `panic`, l'indexation et la comparaison de flottants sont
signalés dans le code de production — et explicitement autorisés dans les
tests, où paniquer est le comportement attendu.

`scripts/verifier.ps1` enchaîne format, clippy, tests Rust et vérification
TypeScript. C'est ce qui doit passer avant tout commit.

## Ajouter un outil à la suite

1. `crates/derovia-<outil>/` — le moteur métier, testé, sans Tauri.
2. `apps/derovia-<outil>/` — l'app Tauri, sur le modèle d'Arbitrage.
3. Ajouter les deux aux `members` du workspace Cargo ; npm les prend
   automatiquement via `workspaces`.
4. Une ligne dans `src/suite.ts`. L'écran de lancement et la barre latérale
   se construisent tous deux à partir de ce catalogue : il n'y a pas de second
   endroit à retoucher.

Si l'outil a besoin d'un composant visuel qui n'existe pas, il va dans
`@derovia/design` — jamais dans la feuille de style de l'outil.
