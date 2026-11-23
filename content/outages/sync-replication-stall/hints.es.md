## 1

Las dos bases de datos siguieron en marcha durante la partición, y las lecturas siguieron funcionando. Solo fallaron las escrituras. ¿A qué espera un commit antes de volver?

## 2

Con `sync = sync` cada commit espera a la única réplica. Cuando esa réplica queda aislada o cae, ningún commit puede terminar, así que perder cualquiera de las dos zonas detiene todas las escrituras. Un commit necesita una confirmación que sobreviva a perder una zona cualquiera.

## 3

Añade una tercera tarjeta PostgreSQL en una nueva AZ-c, replicada desde el primario, y pon `sync = quorum` en todas las tarjetas del clúster: así un commit espera a cualquiera de las dos réplicas. No hace falta cambiar nada más.
