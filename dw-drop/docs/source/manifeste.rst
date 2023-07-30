Manifeste
=========

Le *manifeste* (fichier ``AndroidManifest.xml`` à la racine d'une
application) est un fichier obligatoire dans tous les applications
Android (`Android Developer Documentation
<https://developer.android.com/guide/topics/manifest/manifest-intro>`_). Son
contenu consiste en un semble de déclarations d'éléments qui
gouvernent le bon fonctionnement d'une application ainsi que ses
interactions avec le système Android et les autres applications
installées.

Dans la suite de cette section nous allons parcourir les différents
éléments du manifeste accessibles et modifiables avec *dw-drop*.


Application
-----------

- ``allow_backup``:

  - ``true`` (valeur par défaut) autorise l'inclusion les données de
    l'application lors d'une sauvegarde complète du système,
  - ``false`` empêche l'inclusion des données de l'application lors
    d'une sauvegarde complète du système;

- ``allow_clear_user_data``:

  - ``true`` (valeur par défaut) autorise l'utilisateur ou le système
    à effacer les données de l'application,
  - ``false`` empêche l'utilisateur ou le système d'effacer les
    données de l'application (en cas de *wipe* de la partition *user*
    les données de l'application seront tout de même effacées);

- ``debuggable``:

  - ``true`` autorise l'observation et l'instrumentation de
    l'application par les outils de débogage,
  - ``false`` (valeur par défaut) empêche l'observation et
    l'instrumentation de l'application par les outils débogage;

- ``uses_cleartext_traffic``, à partir d'Android 7/API 24 cet attribut
  n'est honoré que si aucune *configuration de sécurité réseau* (voir
  :ref:`network-security-config`) n'est présente:

  - ``true`` (valeur par défaut jusqu'à l'API 27) autorise
    l'application à effectuer des communications réseaux non chiffrées
    (par exemple en HTTP ou en FTP),
  - ``false`` (valeur par défaut à partir de l'API 28) autorise
    l'application à effectuer des communications réseaux non chiffrées
    (par exemple en HTTP ou en FTP).



Activités
---------

Une activité représente un "écran" d'interface utilisateur dans une
application. Pour être utilisable/affichable, une activité doit être
nécessairement être déclarée dans le manifeste. *dw-drop* permet de
supprimer la déclaration d'activités afin de limiter (du côté de
l'interface utilisateur) certaines fonctionnalités non désirées de
l'application. Chaque activité est identifiée par le nom complet de la
classe qui correspond et qui hérite (directement ou indirectement)
de la classe ``Activity``.

  **Remarque**: une application qui tente d'afficher une activité qui
  n'est pas/plus déclarée dans le manifeste est immédiatement arrêtée par
  le système.

L'affichage de certaines activités peut être déclenché par d'autres
applications. C'est le cas des activités *exportées* (marquées d'un
badge jaune "Exporté" dans *dw-drop*). Le principe des activités
exportées est de permettre à l'application d'ouvrir un "contenu"
(adresse web, image, invitation de réunion, *etc.*) ou d'être
destinatrice d'un contenu partagé qui nécessite d'afficher un "écran"
spécifique. Par exemple, la plupart des applications de messagerie
instantanée disposent d'une activité exportée qui leur permet de
recevoir un document (image, vidéo, *etc.*) à partager à un ou
plusieurs destinataires. L'activité exportée propose alors à
l'utilisateur de choisir les destinataires puis d'envoyer le contenu
avant de rendre la main à l'application appelante.

.. Les activités listées correspondent à des déclarations ``activity`` ou ``activity-alias`` dans le manifeste de l'application, qu'elles soient déclarées comme actives (attribut ``enabled``) ou non.


Composantes
-----------

Les composantes (*features*) représentent le besoin optionnel (ou la
nécessité lorsque l'attribut ``required`` est positionné et qui est
visible par la présence d'un badge jaune "Requis" dans l'interface de
*dw-drop*) par l'application de fonctionnaltés du terminal pour
pouvoir fonctionner, comme par exemple un appareil photo
(``android.hardware.camera``), un lecteur NFC
(``android.hardware.nfc``) ou encore l'ensemble des fonctionnalités
liées à la téléphonie (``android.hardware.telephony``).

  **Remarque**: les composantes ne sont pas liées aux permissions, elles
  sont uniquement utilisées pour gérer l'aspect fonctionnel d'une
  application. Une application qui déclare avoir besoin que le terminal
  dispose d'un appareil photo (composante ``android.hardware.camera``)
  pour fonctionner, elle ne pourra pas y accéder directement qu'à
  condition que la permission ``android.permission.CAMERA`` lui soit
  accordée.

.. Les activités listées correspondent à des déclarations ``uses-feature``, qu'elles soient déclarées comme requises ou non.


Permissions
-----------

Les permissions nécessaires à une application pour accéder à certaines
ressources du terminal sur lequel s'exécute l'application doivent être
déclarées dans le manifeste.

Lorsque l'on supprime une permission du manifeste cela permet de
s'assurer que l'application ne pourra jamais obtenir cette
permission. Par exemple, certaines applications peuvent vouloir
accéder aux données de géolocalisation de manière injustifiée. Dans ce
cas, supprimer les permissions
``android.permission.ACCESS_COARSE_LOCATION``,
``android.permission.ACCESS_FINE_LOCATION`` et
``android.permission.ACCESS_BACKGROUND_LOCATION`` permet de se
prémunir contre l'utilisation de ces données par l'application, y
compris par exemple aux bibliothèques de publicité embarquées.

.. Les permissions listées correspondent à des déclarations ``uses-permission`` ou ``uses-permission-sdk-23``.

  **Remarque**: les permissions qui figurent dans le manifeste
  correspondent aux permissions que l'application *peut* demander,
  pas aux permissions accordées *in fine*; cela reste de la
  responsabilité de l'utilisateur d'accorder ou non les dites
  permissions.


Fournisseurs
------------

Les fournisseurs de données (*providers*) permettent à une application
d'être une sources de données pour d'autres applications qui la
sollicite. Par exemple, une application qui gère une galerie de photos
peut proposer un fournisseur de données de type "image" aux autres
applications afin de leur permettre d'y accéder. Seuls les
fournisseurs de données *exportés*, marqués par un badge jaune
"Exporté" dans l'interface de *dw-drop*, sont accessibles aux autres
applications.

L'accès à un fournisseur de données peut être contrôlé par une
permission *ad hoc*, mais la plupart du temps il ne l'est pas.

  **Remarque**: la suppression d'un fournisseur de données ne créé pas
  de problème fonctionnel (plantage/crash). Cela permet en revanche
  d'éviter de tenter un utilisateur de partager des données gérées par
  l'application modifiée.


Récepteurs
----------

Les récepteurs de diffusion (*receiver*) permettent à une application
de recevoir des informations/données diffusées par le système Android
(par exemple: le téléphone a démarré, le réseau est disponible, un
SMS/MMS est arrivé, etc.) ou par d'autres applications. Seuls les
récepteurs *exportés*, marqués par un badge jaune "Exporté" dans
l'interface de *dw-drop*, sont accessibles aux autres applications.

.. listés correspondent à des déclarations ``receiver`` du manifeste, qu'ils soient déclarés comme actifs (attribut ``enabled``) ou non.

  **Remarque**: la suppression d'un récepteur ne créé pas de
  plantage/crash, mais peut occasionner des dysfonctionnements
  fonctionnels. Par exemple, supprimer un récepteur qui attend de
  recevoir l'information qui indique que le téléphone a démarré peut
  empêcher le démarrage de services de l'application nécessaires à son
  bon fonctionnement. Dans le cas d'une application de messagerie
  instantanée, par exemple, celle-ci ne se mettra pas en écoute de
  nouveaux messages automatiquement au (re)démarrage du téléphone; il
  faudra que l'utilisateur lance manuellement l'application pour
  commencer à recevoir les messages. Supprimer certains récepteurs
  permet d'éviter à une application (potentiellement indiscrète) d'être
  notifiée de certains événements.


Services
--------

Les services correspondent à du code d'une application qui s'exécute
en tâche de fond, même lorsque l'application n'est pas au premier
plan. Comme pour les activités, seuls les services déclarés dans le
manifeste peuvent être démarrés.

Les services en tâche de fond sont très employés sur Android en
particulier pour les applications qui peuvent sans les Google Play
Services. En effet, en l'absence des Google Play Services sur le
terminal une application ne pourra pas recevoir de notifications
lorsque'elle se trouve en arrière plan sans disposer de son propre
service d'écoute en arrière plan. C'est le cas de la plupart des
applications de messagerie instantanées, par exemple, qui ont besoin
de notifier l'utilisateur de l'arrivée de messages sans avoir besoin
de lancer la dite application.

Les services *exportés*, marqués par un badge jaune "Exporté" dans
l'interface de *dw-drop*, sont accessibles par les autres
applications.
.. Les conditions d'accès à ces services ne sont pas affichés dans *dw-drop*.

  **Remarque**: une application qui tente de démarrer un service qui
  n'est pas/plus déclaré dans le manifeste est immédiatement arrêtée par
  le système. 

