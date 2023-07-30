Signature d'application
=======================

Le système Android requiert que toute application soit signée avec un
certificat pour être installée sur un terminal. Puisque *dw-drop* modifie les
applications, il est nécessaire de les signer à nouveau. Une interface prévue
à cet effet est intégrée à *dw-drop*. Il s'agit d'une interface graphique qui
fait appel à l'outil ``apksigner``, directement embarqué dans *dw-drop* et
faisant partie de la suite d'outil du SDK Android.