.. _network-security-config:

Configuration de la sécurité réseau
===================================

La *configuration de la sécurité réseau* (fichier
``res/xml/network_security_config.xml`` dans les applications, mais son
nom peut avoir être offusqué) est un fichier optionnel dans les
applications Android qui permet de restreindre/contrôler les flux
HTTP(S) établis via la bibliothèque standard d'Android (`Android -
Network Security Configuration
<https://developer.android.com/training/articles/security-config>`_).

Plus spécifiquement, ce fichier permet de spécifier si une application :

- est autorisée à établir des connexions HTTP (non
  chiffrées) ;

- est autorisée à établir des connexions HTTPS (chiffrées), et vers
  quel(s) domaine(s) en précisant pour chacun les autorités de
  certification acceptées (liste blanche) pour le(s) certificat(s)
  serveur(s).

*dw-drop* permet de consulter, lorsqu'elle existe, la configuration de
sécurité réseau en place dans l'application. *dw-drop* permet
également de supprimer une configuration existante, de remplacer une
configuration existante par des configurations "standard" prédéfinies
et d'ajouter une configuration lorsqu'aucune n'est définie.

Les configurations prédéfinies dans *dw-drop* sont :

- "Magasin système, trafic en clair non autorisé": les autorités de
  certification autorisées sont toutes celles présentes et autorisées
  dans le magasin (immuable) de certificats d'Android, les
  communications HTTP (non chiffrées) ne sont pas autorisées ;

- "Magasins système et utilisateur, trafic en clair non autorisé": :
  les autorités de certification autorisées sont toutes celles
  présentes et autorisées dans le magasin (immuable) de certificats
  d'Android et dans le magasin utilisateur (modifiable), les
  communications HTTP (non chiffrées) ne sont pas autorisées ;

- "Magasin système, trafic en clair autorisé": les autorités de
  certification autorisées sont toutes celles présentes et autorisées
  dans le magasin (immuable) de certificats d'Android, les
  communications HTTP (non chiffrées) sont autorisées ;

- "Magasins système et utilisateur, trafic en clair autorisé":
  les autorités de certification autorisées sont toutes celles
  présentes et autorisées dans le magasin (immuable) de certificats
  d'Android et dans le magasin utilisateur (modifiable), les
  communications HTTP (non chiffrées) sont autorisées ;

- "Aucune": la configuration dépend de la version d'Android du
  terminal, se référer à la `documentation Android - Network Security
  Configuration
  <https://developer.android.com/training/articles/security-config>`_.
