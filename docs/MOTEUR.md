# Le modèle de calcul

Ce document explique ce que compare Derovia Arbitrage, et surtout **pourquoi**
la comparaison est menée ainsi. Le code vit dans
[`crates/derovia-arbitrage/src/engine.rs`](../crates/derovia-arbitrage/src/engine.rs).

## Le problème

Comparer une mensualité de crédit à un loyer ne veut rien dire. Trois raisons :

1. Une partie de la mensualité rembourse du capital — c'est de l'épargne, pas
   une dépense.
2. L'apport immobilisé aurait pu être placé ailleurs et rapporter.
3. Le bien prend ou perd de la valeur pendant qu'on le détient.

Un outil qui ignore l'un de ces trois points donne systématiquement tort à la
location.

## La méthode : même budget, place la différence

Le moteur simule **deux personnes disposant du même budget mensuel**. L'une
achète, l'autre loue.

Chaque mois :

- on calcule le coût réel de chaque option ;
- le budget commun est celui de l'option la plus chère ;
- celle qui coûte moins cher **place la différence** au taux de rendement du
  marché.

À l'horizon, on totalise pour chacune :

```
patrimoine net = capital placé + valeur de revente du bien − dette restante
```

C'est l'écart entre ces deux patrimoines qui tranche. Sans le mécanisme de
placement de la différence, la location paraîtrait toujours perdante — c'est
l'erreur classique des comparateurs.

## Ce qui entre dans chaque option

**Acheter** — prix, frais d'acquisition (notaire, carte grise, installation),
apport, mensualité de crédit, assurance emprunteur, entretien annuel, charges
fixes, évolution de la valeur, décote immédiate, frais de revente.

**Louer** — loyer indexé, charges non incluses, frais d'entrée non
récupérables, dépôt de garantie (immobilisé puis rendu), option d'achat
éventuelle.

Le dépôt de garantie n'est pas compté comme une dépense : il est immobilisé,
donc il ne rapporte rien, mais il revient à la sortie. Les deux effets sont
modélisés séparément.

## Conventions de calcul

| Point | Convention | Pourquoi |
|---|---|---|
| Taux de crédit | Proportionnel, `taux / 12` | C'est la convention bancaire française |
| Rendement du placement | Actuariel, `(1+taux)^(1/12) − 1` | Douze mois composés redonnent exactement le taux annuel |
| Pas de simulation | Mensuel | Un prêt s'amortit au mois ; l'annuel accumulerait l'erreur |
| Charges | Indexées sur l'inflation | Une taxe foncière de 2024 n'est pas celle de 2044 |
| Valeur du bien | `prix × (1 − décote immédiate) × (1 + évolution)^années` | Couvre l'immobilier qui monte comme le véhicule qui s'effondre |

## Fiscalité professionnelle

Activée uniquement pour le preset « Matériel pro ». Un particulier n'y a pas
droit, et le lui appliquer fausserait tout.

- **TVA récupérable** : les montants saisis sont compris comme TTC, le moteur
  raisonne sur leur équivalent HT, des deux côtés.
- **Amortissement** : `prix HT / durée` déduit chaque année, converti en
  économie d'impôt au taux d'imposition du bénéfice.
- **Charges déductibles** : intérêts, assurance, entretien côté achat ; loyers
  côté leasing.

## Ce que le moteur refuse de faire

**Trancher quand l'écart est dans le bruit.** En dessous de 3 % d'écart relatif,
la recommandation est « Trop serré ». Un avantage de 2 % sur vingt ans se joue
entièrement dans la marge d'erreur des hypothèses ; annoncer une décision ferme
serait mentir sur la précision du calcul.

## Les deux chiffres qui comptent vraiment

**Le point de bascule** — la première année où acheter repasse devant. C'est ce
que les gens retiennent : « il faut rester au moins quinze ans ».

**Le seuil de rendement** (`indifference_return`) — le rendement que doit
atteindre le capital placé pour renverser la décision. C'est la mesure de
robustesse de la conclusion : un seuil à 10 % veut dire que le verdict est
solide, un seuil à 4 % qu'il ne tient à rien.

## Un résultat contre-intuitif, et vrai

On croit généralement qu'allonger l'horizon finit toujours par donner raison à
l'achat, parce que les frais d'acquisition se diluent. **C'est faux dès que le
capital placé croît plus vite que le bien.** Les frais s'amortissent une fois,
mais un écart de taux ne s'amortit jamais : il compose. Passé le seuil de
rendement, plus on attend, plus la location gagne.

Le test
[`past_the_indifference_threshold_time_stops_helping_the_buyer`](../crates/derovia-arbitrage/tests/engine.rs)
verrouille ce comportement.

## Les hypothèses par défaut

Les presets donnent des ordres de grandeur du marché français, destinés à être
ajustés. **Ce ne sont pas des conseils en investissement.**

Le paramètre le plus décisif de tout l'outil est l'évolution de la valeur du
bien : un point de plus ou de moins renverse la conclusion sur vingt ans. Il est
fixé à 2,5 %/an pour le logement — tendance longue, hors cycles.
