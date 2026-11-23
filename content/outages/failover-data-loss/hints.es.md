## 1

La conmutación tardó unos diez segundos y después los errores pararon, pero la prueba sigue fallando. Mira el recuento de pérdidas: ¿qué pagos se confirmaron a los clientes pero nunca llegaron al nuevo primario?

## 2

El primario confirmaba cada commit antes de que lo tuviera ninguna réplica, y las réplicas van unos 100 ms por detrás. Cuando murió, la réplica que tomó el relevo nunca había visto sus últimos 100 ms de pagos. Un commit que no se puede perder tiene que esperar a que otro miembro también lo tenga.

## 3

Pon `sync` en `quorum` en las tres tarjetas de PostgreSQL y mantenlas en tres AZ. Así un commit espera a una mayoría del clúster, de modo que cualquier miembro puede morir sin perder un pago confirmado, y los dos que quedan siguen siendo mayoría, así que los commits continúan tras la conmutación.
