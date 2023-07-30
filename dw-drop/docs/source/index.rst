Bienvenue sur la documentation de dw-drop !
===========================================

*dw-drop* est un outil d'analyse et de manipulation d'applications
Android. *dw-drop* permet d'apporter les modifications suivantes
directement sur des applications au format APK provenant des magasins
d'applications (par exemple, du Google PlayStore) :

- retrait de permissions ;
- (dés)activation d'activités, de services, de fournisseurs, etc. ;
- (dés)activation de la sauvegarde des données de l'application lors d'un *backup* du téléphone/tablette ;
- (dés)activation de la possibilité de débogage ;
- (dés)activation de la possibilité d'effectuer du trafic réseau *en clair* (*c.-à-d.* en HTTP plutôt qu'en HTTPS)
- altération de la politique de sécurité réseau (changement des autorités de certifications autorisées pour les connexions HTTPS) ;
- retrait/modification/remplacement des fichiers embarqués dans l'application (images, bibliothèques natives).

*dw-drop* permet également de signer des applications au format APK en utilisant un magasin de clés (PKCS12, protégé ou non par mot de passe) ou directement une clé au format PKCS8 (protégée ou non par mot de passe).

Le sommaire ci-dessous couvre plus en détails les différentes fonctionnalités de l'outil.

.. toctree::
   :maxdepth: 2
   :caption: Sommaire :

   manifeste
   fichiers
   dex
   nsc
   signature
