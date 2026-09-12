# Brief : section de téléchargement sur le site Derovia

> À transmettre tel quel à l'IA qui développe le site.

---

## Ce qu'il faut ajouter

Une section de téléchargement pour **Derovia**, une suite d'outils de bureau pour
Windows. Un bouton principal, les informations qui rassurent avant de cliquer, et
**un avertissement indispensable** détaillé plus bas.

## Le lien de téléchargement

```
https://github.com/suuuucdcz/derovia-suite/releases/latest/download/Derovia-setup.exe
```

Ce lien pointe toujours vers la dernière version publiée : il n'y aura jamais à le
modifier lors d'une mise à jour.

## Informations à afficher près du bouton

- **Windows 10 ou 11, 64 bits**
- **Installeur de 2,6 Mo**
- Version actuelle : **0.1.0**
- Gratuit

## ⚠️ Le point le plus important : l'avertissement Windows

L'application **n'est pas signée numériquement**. Au lancement de l'installeur,
Windows affiche un écran bleu :

> **Windows a protégé votre ordinateur**
> Microsoft Defender SmartScreen a empêché le démarrage d'une application non reconnue.

**Sans explication sur la page, la majorité des visiteurs abandonnent à cet
écran.** Il faut donc afficher, juste sous le bouton, un encadré visible mais
rassurant :

> **Windows affichera un avertissement au premier lancement.**
> C'est normal : Derovia est une application récente, pas encore référencée par
> Microsoft. Cliquez sur **Informations complémentaires**, puis sur **Exécuter
> quand même**.

Idéalement, accompagner d'une capture d'écran de cet avertissement avec les deux
boutons entourés. C'est ce qui fait la différence entre un visiteur qui installe
et un visiteur qui ferme l'onglet.

Ne pas minimiser ni cacher cette information : un utilisateur surpris par un
avertissement de sécurité non annoncé fait davantage demi-tour qu'un utilisateur
prévenu.

## Ce que fait le logiciel

Trois outils dans une seule application :

- **Arbitrage** — calcule s'il vaut mieux acheter ou louer un logement, un
  véhicule ou du matériel professionnel. Donne le point de bascule en années.
- **Convertisseur** — convertit documents et images : Word, PDF, Markdown, HTML,
  texte, PNG, JPEG, WebP.
- **Compresseur** — réduit le poids des images, PDF et autres fichiers.

## Premier lancement

Au premier démarrage, l'application télécharge automatiquement son moteur de
conversion (environ 40 Mo, une à deux minutes). Une barre de progression
l'indique, et l'application reste utilisable pendant ce temps.

Le mentionner brièvement évite l'inquiétude : « À la première ouverture, Derovia
prépare ses moteurs de conversion — comptez une minute. »

## Ce qu'il ne faut PAS écrire

Ces formulations seraient fausses :

- ❌ « Fonctionne entièrement hors ligne » — le premier lancement et la
  synchronisation des comptes demandent une connexion.
- ❌ « Ouvre tous les formats Word » — les `.doc` de Word 97-2003 exigent un
  moteur complémentaire, proposé dans l'application.
- ❌ « Application certifiée / vérifiée » — elle n'est pas signée.
- ❌ « Compatible Mac / Linux » — Windows 64 bits uniquement.

Ce qui est vrai et mérite d'être dit : **les fichiers traités ne quittent jamais
la machine de l'utilisateur.** Toute la conversion et la compression se font en
local.

## Ton et présentation

Le logiciel a une identité visuelle sobre, inspirée des logiciels Apple : formes
arrondies généreuses, beaucoup de blanc, une seule couleur d'accent (bleu
`#0071e3`), typographie système. La section de téléchargement devrait rester dans
ce registre — pas de dégradés criards, pas d'animation tape-à-l'œil.

Éviter le vocabulaire marketing survendu (« révolutionnaire », « le meilleur »).
Le produit se décrit par ce qu'il fait.

## Structure suggérée

1. Titre court et le nom
2. Une phrase qui dit ce que c'est
3. Le bouton de téléchargement, bien visible
4. Configuration requise et poids, en petit
5. **L'encadré d'avertissement Windows**
6. Les trois outils, en trois cartes
7. La mention du premier lancement
