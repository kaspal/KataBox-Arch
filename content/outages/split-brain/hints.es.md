## 1

Durante la partición, los dos lados de la base de datos podían aceptar escrituras. ¿Qué lado tenía permiso para reservar existencias, y qué lo decidía?

## 2

Un bloqueo en Redis es para quien llegue a Redis; no sabe que hay una partición. Solo un bloqueo que necesita una mayoría de sus miembros puede rechazar al lado que quedó aislado. Y la base de datos también debería rechazar las escrituras en un primario que se promovió a sí mismo.

## 3

Sustituye el bloqueo en Redis por un servicio de coordinación etcd: tres tarjetas con 1 instancia cada una, una en la AZ-a, otra en la AZ-b y otra en una nueva AZ-c, unidas con aristas Replicate desde la de la AZ-a, y conecta las dos aplicaciones a la de la AZ-a. Activa los tokens de exclusión (fencing) en las dos tarjetas de PostgreSQL. Deja la conmutación en una partición en automático.
