# Publier une nouvelle version

## En une phrase

Changez le numéro de version, posez une étiquette, poussez : GitHub compile,
signe, publie, et les applications déjà installées proposent la mise à jour.

```bash
git tag v0.3.0
git push origin v0.3.0
```

L'onglet **Actions** du dépôt montre l'avancement. Une dizaine de minutes plus
tard, la release est en ligne.

## Ce qu'il faut changer avant de poser l'étiquette

Un seul endroit : `apps/derovia-arbitrage/src-tauri/tauri.conf.json`, la ligne
`"version"`. Elle doit correspondre à l'étiquette, sans le `v` :

| Étiquette | `"version"` |
| --------- | ----------- |
| `v0.3.0`  | `0.3.0`     |

Si les deux ne correspondent pas, la mise à jour ne se déclenchera pas :
l'application compare son propre numéro à celui annoncé, et deux versions
identiques ne donnent rien à installer.

## La clé de signature — à sauvegarder maintenant

Le fichier `.secrets/derovia-update.key` **n'est pas dans le dépôt** et ne peut
pas l'être : quiconque l'obtiendrait pourrait envoyer sa propre application à
tous les utilisateurs de Derovia, qui l'installeraient sans un avertissement.

Il est aussi **irremplaçable**. La clé publique correspondante est compilée dans
chaque installation déjà distribuée ; sans la clé privée, aucune mise à jour ne
sera plus jamais acceptée par ces installations. Il faudrait demander à chaque
utilisateur de désinstaller et de réinstaller à la main.

> **À faire une fois, aujourd'hui :** copiez `.secrets/derovia-update.key` et
> `.secrets/derovia-update.key.pub` ailleurs que sur ce disque — gestionnaire de
> mots de passe, clé USB rangée, coffre-fort en ligne. Un disque qui tombe en
> panne suffit à la perdre.

## Les deux secrets à déposer sur GitHub

Une seule fois, pour que la compilation automatique puisse signer.

1. Ouvrez `https://github.com/suuuucdcz/derovia-suite/settings/secrets/actions`
2. **New repository secret**, deux fois :

| Nom                                  | Valeur                                                   |
| ------------------------------------ | -------------------------------------------------------- |
| `TAURI_SIGNING_PRIVATE_KEY`          | tout le contenu du fichier `.secrets/derovia-update.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | laisser vide                                              |

Pour lire le contenu du fichier, ouvrez-le avec le Bloc-notes et copiez tout.

Sans ces secrets, la compilation s'arrête d'elle-même avec le message
« Aucune signature produite » — plutôt que de publier une version que personne
ne pourrait installer en mise à jour.

## Ce que voit l'utilisateur

Trois secondes après l'ouverture de Derovia, si une version plus récente
existe, un bandeau discret apparaît en bas de la fenêtre. « Plus tard » le fait
disparaître. « Installer » télécharge, vérifie la signature, installe et
relance.

Rien n'est imposé, et une vérification qui échoue — réseau coupé, GitHub
indisponible — passe inaperçue : l'outil reste utilisable.

## Le lien du site ne change jamais

```
https://github.com/suuuucdcz/derovia-suite/releases/latest/download/Derovia-setup.exe
```

Chaque publication dépose l'installeur sous ce même nom. Le bouton du site
n'a donc jamais besoin d'être modifié.
