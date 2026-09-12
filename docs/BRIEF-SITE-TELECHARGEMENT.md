# Brief : section de téléchargement de Derovia

> Document à transmettre tel quel à l'IA qui développe le site.
> Tout ce qui suit a été vérifié sur les fichiers réellement en ligne.
> Dernière vérification : 12 septembre 2026, version 0.2.2.

---

## 1. Ce qu'il faut construire

Une section de téléchargement pour **Derovia**, une suite d'outils de bureau
pour Windows. Un bouton principal, les informations qui rassurent avant de
cliquer, et **un encadré d'avertissement sans lequel la moitié des visiteurs
abandonneront** (section 4 — c'est le point le plus important de ce document).

## 2. Le lien de téléchargement

```
https://github.com/suuuucdcz/derovia-suite/releases/latest/download/Derovia-setup.exe
```

Ce lien pointe toujours vers la dernière version publiée. **Il ne faudra jamais
le modifier**, y compris lors des mises à jour : le nom de fichier ne change
pas d'une version à l'autre. Ne pas y ajouter de numéro de version, ne pas le
remplacer par un lien vers une version précise.

Un simple `<a href="…" download>` suffit. Pas de formulaire préalable, pas de
redirection intermédiaire, pas de compteur : le fichier est servi directement
par GitHub.

## 3. Informations à afficher près du bouton

- **Windows 10 ou 11, 64 bits**
- **Installeur de 2,9 Mo**
- Version actuelle : **0.2.2**
- Gratuit
- Se met à jour automatiquement

## 4. ⚠️ L'avertissement Windows — le point décisif

L'application **n'est pas signée numériquement**. Au lancement de l'installeur,
Windows affiche un écran bleu pleine largeur :

> **Windows a protégé votre ordinateur**
> Microsoft Defender SmartScreen a empêché le démarrage d'une application non
> reconnue. L'exécution de cette application peut mettre votre ordinateur en
> danger.

Le bouton visible est **« Ne pas exécuter »**. Le bouton qui permet de
continuer est caché derrière un lien discret, **« Informations
complémentaires »**.

**Sans explication sur la page, la majorité des visiteurs s'arrêtent là.** Il
faut donc afficher, juste sous le bouton de téléchargement, un encadré visible
mais rassurant :

> **Windows affichera un avertissement au premier lancement.**
> C'est normal : Derovia est une application récente, pas encore référencée par
> Microsoft. Cliquez sur **Informations complémentaires**, puis sur **Exécuter
> quand même**.

Trois règles sur cet encadré :

- **Le placer avant le clic, pas après.** Un utilisateur surpris par un
  avertissement de sécurité non annoncé fait demi-tour bien plus souvent qu'un
  utilisateur prévenu.
- **Ne pas le minimiser** derrière un « en savoir plus » replié, ne pas le
  mettre en gris pâle, ne pas le reléguer en bas de page.
- **Ne pas le dramatiser non plus.** Ton factuel, pas d'icône de danger rouge.

Idéalement, l'accompagner d'une capture d'écran de l'avertissement avec les
deux boutons à cliquer entourés. C'est ce qui fait la différence entre un
visiteur qui installe et un visiteur qui ferme l'onglet.

## 5. Ce que fait le logiciel

Trois outils dans une seule application :

- **Arbitrage** — calcule s'il vaut mieux acheter ou louer un logement, un
  véhicule ou du matériel professionnel. Donne le point de bascule en années.
- **Convertisseur** — convertit documents et images : Word, PDF, Markdown,
  HTML, texte, PNG, JPEG, WebP.
- **Compresseur** — réduit le poids des images, PDF et autres fichiers.

Trois autres sont annoncés dans l'application comme *Bientôt* : Prévision,
Devis, Veille. **Ne pas les présenter comme disponibles.**

## 6. Le premier lancement

Au premier démarrage, l'application télécharge son moteur de conversion :
**environ 40 Mo, une à deux minutes**. Une barre de progression l'indique, et
l'application reste utilisable pendant ce temps.

Une phrase suffit sur la page : « À la première ouverture, Derovia prépare ses
moteurs de conversion — comptez une minute. »

Un second moteur, optionnel et bien plus lourd, est proposé **à l'intérieur de
l'application** pour les documents les plus exigeants. Il n'a pas à figurer sur
le site. **S'il devait être mentionné, l'appeler « moteur haute fidélité » —
jamais par le nom du logiciel tiers sur lequel il repose.**

## 7. Les mises à jour

À partir de la version 0.2.0, Derovia vérifie au démarrage s'il existe une
version plus récente et propose de l'installer. La notification est discrète et
se reporte d'un clic ; rien n'est imposé.

Conséquences pour le site :

- **pas de page « historique des versions » à tenir** ;
- **pas de lien à mettre à jour** lors d'une nouvelle sortie ;
- le numéro de version affiché en section 3 peut rester tel quel entre deux
  sorties sans que rien ne casse.

## 8. Ce qu'il ne faut PAS écrire

Ces formulations seraient factuellement fausses :

- ❌ « Fonctionne entièrement hors ligne » — le premier lancement, les mises à
  jour et la synchronisation des comptes demandent une connexion.
- ❌ « Ouvre tous les formats Word » — les `.doc` de Word 97-2003 exigent le
  moteur complémentaire, proposé dans l'application.
- ❌ « Application certifiée / vérifiée / signée » — elle ne l'est pas, et
  c'est précisément ce que dit l'avertissement Windows.
- ❌ « Compatible Mac / Linux » — Windows 64 bits uniquement.
- ❌ Toute mention du nom du logiciel tiers derrière le moteur haute fidélité.

Ce qui est vrai et mérite d'être dit : **les fichiers traités ne quittent
jamais la machine de l'utilisateur.** Toute la conversion et toute la
compression se font en local.

## 9. Ton et présentation

Le logiciel a une identité visuelle sobre, inspirée des logiciels Apple :
formes arrondies généreuses, beaucoup de blanc, une seule couleur d'accent
(bleu `#0071e3`), typographie système. La section de téléchargement doit rester
dans ce registre — pas de dégradés criards, pas d'animation tape-à-l'œil, pas
de compte à rebours ni de « offre limitée ».

Éviter le vocabulaire marketing survendu (« révolutionnaire », « le meilleur »,
« puissant »). Le produit se décrit par ce qu'il fait.

## 10. Structure suggérée

1. Le nom, et un titre court
2. Une phrase qui dit ce que c'est
3. Le bouton de téléchargement, bien visible
4. Configuration requise et poids, en petit, sous le bouton
5. **L'encadré d'avertissement Windows** (section 4)
6. Les trois outils, en trois cartes
7. La mention du premier lancement (section 6)

## 11. Récapitulatif des valeurs exactes

| Élément | Valeur |
| --- | --- |
| Lien de téléchargement | `https://github.com/suuuucdcz/derovia-suite/releases/latest/download/Derovia-setup.exe` |
| Nom du fichier téléchargé | `Derovia-setup.exe` |
| Poids de l'installeur | 2 896 651 octets, soit 2,9 Mo |
| Version actuelle | 0.2.2 |
| Systèmes | Windows 10 et 11, 64 bits |
| Prix | gratuit |
| Signature numérique | aucune — avertissement SmartScreen au premier lancement |
| Téléchargement au 1er lancement | ~40 Mo (moteur de conversion) |
| Mises à jour | automatiques, proposées depuis l'application |
