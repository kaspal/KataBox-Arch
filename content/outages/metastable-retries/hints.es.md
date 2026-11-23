## 1

El pico duró diez segundos, pero los errores no pararon cuando terminó. ¿En qué seguía ocupado el servicio Profile después de los 25 s, y quién esperaba esas respuestas?

## 2

La cola de Profile podía contener unos 250 ms de trabajo, más que el timeout de 200 ms de la malla. Las llamadas expiraban mientras seguían en la cola, el servicio hacía el trabajo igualmente, y la malla volvía a enviar cada una, tres veces. El trabajo que espera más que el timeout de su llamador no le sirve a nadie.

## 3

En el servicio Profile, pon Descartar carga con cola de más de en 8. Deja los timeouts y los reintentos como están: el exceso ahora se rechaza al instante en lugar de acumularse.
