# Le design system

Arrondi mais professionnel. La référence est le logiciel Apple : des formes
généreuses, beaucoup de blanc, une seule couleur d'accent, et des chiffres qui
ne bougent pas quand ils changent.

## Les quatre feuilles

| Fichier | Rôle |
|---|---|
| `tokens.css` | Toutes les valeurs : couleurs, corps, rayons, ombres, durées |
| `base.css` | Remise à zéro du navigateur, typographie, focus, ascenseurs |
| `components.css` | Le vocabulaire : cartes, champs, boutons, tableau, graphique |
| `launcher.css` | L'accueil de la suite : marque, grille d'outils, fenêtre modale |

L'ordre compte : les jetons d'abord. `index.css` s'en charge.

L'écran de lancement appartient au système, pas à un outil : chaque outil de
Derovia ouvre sur le même accueil. Le jour où la suite aura son propre lanceur
— un binaire qui démarre les autres — `launcher.css` sera son interface sans
être réécrit.

## Les règles

**Aucune valeur en dur.** Pas un `#fff`, pas un `12px` dans une feuille
d'outil. Tout passe par une variable — c'est ce qui permet de changer la charte
d'un coup, et ce qui fait que le thème sombre marche sans travail
supplémentaire.

**Un outil ne redéfinit que `--accent`.** Sur sa racine, en une ligne.

**Les composants appartiennent au système.** Si un outil a besoin d'un
composant, il va dans `components.css`, pas dans sa feuille locale. Sinon la
suite se disloque au troisième outil.

## Les choix, et leur raison

**Polices système.** SF Pro sur macOS, Segoe UI Variable sur Windows 11. Aucun
téléchargement, donc aucun saut de rendu au premier affichage — et les deux ont
la même sobriété géométrique.

**Le crénage se resserre quand le corps grandit.** C'est la signature des
titres Apple : `--tracking-tight` sur les grands corps, `--tracking-normal` sur
le texte courant.

**Tous les chiffres en chasse fixe.** `font-variant-numeric: tabular-nums`. Les
colonnes de montants s'alignent, et un nombre qui se met à jour ne fait plus
sauter la mise en page.

**Deux ombres superposées, jamais une.** Une nette au ras de l'élément pour le
détacher, une large et diffuse pour l'élévation. Une seule ombre paraît
toujours sale.

**En thème sombre, les surfaces s'éclaircissent avec l'élévation** au lieu de
projeter une ombre : dans le noir, une ombre ne se voit pas, un écart de
luminosité si.

**Une seule courbe de mouvement** pour toute la suite : `--ease`, très
décélérante. Le mouvement démarre franchement puis s'installe, sans rebond.
Quand le système demande moins d'animation, elle est **supprimée**, pas
raccourcie : une transition de 1 ms reste perçue par qui y est sensible.

**Le rayon croît avec la surface** — de `--radius-xs` (6 px) à `--radius-2xl`
(30 px) — pour que la courbure perçue reste constante d'un composant à l'autre.

## Les couleurs sémantiques

`--buy` (bleu) et `--rent` (ambre) désignent les deux options **partout** :
dans le graphique, la légende, le tableau et le verdict. Le couple bleu/ambre
reste distinguable en vision dichromatique, contrairement au couple
rouge/vert.

`--positive`, `--negative` et `--neutral` sont réservés au jugement, jamais à
l'identité d'une série.

## Le graphique

Généré à la main en SVG, sans bibliothèque. Il hérite ainsi directement des
variables de couleur — donc du thème — sans configuration.

Une seule bande est remplie, entre les deux courbes, teintée de la couleur de
celui qui mène. Remplir l'aire sous chaque courbe donnerait deux translucides
superposés, donc du gris sale ; la bande montre exactement ce qu'on cherche à
lire, l'écart.

Les graduations tombent toujours sur 1, 2 ou 5 fois une puissance de dix : un
axe gradué tous les 37 412 € est illisible.

## Accessibilité

La bague de focus est visible au clavier et invisible à la souris
(`:focus-visible`). Les couleurs de texte sur fond respectent 4,5:1 dans les
deux thèmes. Le graphique porte un `aria-label` qui décrit ce qu'il montre.
