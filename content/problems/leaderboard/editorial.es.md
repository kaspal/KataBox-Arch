# Solución comentada

## Adónde va la carga

En el pico, 6 400 solicitudes por segundo leen unos 100 primeros y 1 600 envían una puntuación. Los datos en sí son mínimos: 50 clasificaciones de 100 entradas cada una. Lo difícil es la frecuencia con la que se tocan esos pocos datos y el tamaño de cada lectura. Cada lectura devuelve 32 KB.

Una SQL Database paga por ese tamaño. Un resultado más grande tarda más, aquí cerca de 1,5 veces una consulta pequeña, y retiene una conexión todo ese tiempo. PostgreSQL tiene 100 conexiones, y cada escritura también necesita una.

## El diseño de referencia

Un Load Balancer reparte el tráfico entre cuatro App Servers en Go. Estos leen y escriben en un Cache Redis usado como almacén, sin TTL, así que Redis responde cada lectura por sí mismo y nada caduca. Redis ejecuta cada comando en unos 0,01 ms en un solo hilo, unas 100 000 operaciones por segundo, muy por encima de lo que pide este juego.

Una conexión Replicate une el primario con una réplica, lo que forma un clúster con un tiempo de conmutación por error de 2 s. Cuatro App Servers dejan margen para que el tráfico se duplique. El diseño cuesta unos 341 $ al mes.

## Por qué falla el primer intento

Un primer intento natural mantiene los mismos cuatro App Servers y usa PostgreSQL como almacén. En un día normal pasa con una p95 cercana a 35 ms.

La prueba oculta "Termina el torneo" es donde falla. Durante 20 s, los envíos de puntuación van a cinco veces su ritmo habitual y todo el tráfico se duplica. Las pesadas consultas de los 100 primeros y la avalancha de escrituras compiten por las mismas 100 conexiones. Se agotan, las solicitudes hacen cola y cerca del 33 % falla. Redis absorbe la misma ráfaga sin esfuerzo.

La otra prueba oculta, "El almacén se reinicia", deja caído el primario del almacén durante 10 s y admite hasta un 8 % de errores. Las comprobaciones de salud detectan el primario caído en 2 a 4 s, y la réplica se promueve tras la conmutación de 2 s. Las solicitudes solo fallan en ese hueco: cerca de un 2,7 % de errores en total. Sin réplica, el almacén desaparece los 10 s completos.

## Para probar

- Quita la réplica y vuelve a ejecutar la prueba del reinicio. Compara la tasa de errores con el límite del 8 %.
- Baja a dos App Servers y observa el final del torneo.
- Mantén PostgreSQL pero pon un Cache con un TTL corto delante de las lecturas de los 100 primeros. Comprueba qué sigue llegando a la base de datos cuando termina el torneo.
