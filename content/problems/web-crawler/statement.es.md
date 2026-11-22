# Rastreador web

## El producto

Tienes un rastreador que mantiene una copia al día de páginas web para un producto de búsqueda. Una frontera de URLs espera a ser visitada. Cada URL hay que descargarla del sitio que la aloja y luego guardarla. Descargar es lento, y la web no es tuya: los sitios responden a su ritmo y solo puedes pedir hasta cierto punto. Las páginas se enlazan entre sí todo el tiempo, así que la frontera te sigue entregando URLs que ya visitaste.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Ritmo de rastreo | 800 URLs/s |
| Tiempo de finalización p95 | 4 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 2 500 $ al mes |

## El tráfico

- **Rastrear una URL** (100 %). Un Scheduler, la frontera de rastreo, publica URLs en una Queue. Un Worker toma cada una y puede consultar primero una Cache de URLs vistas. Después descarga la página de la web y la escribe en un almacén. La web responde en unos 250 ms de mediana, con una cola lenta de unos pocos segundos, y admite 1 000 descargas/s. Las llamadas por encima de ese límite fallan de inmediato. Hay 2 millones de URLs, y la frecuencia con que cada una se redescubre sigue una curva Zipf 0,9: algunas vuelven una y otra vez.

La latencia se mide desde que se publica una URL hasta que su trabajo termina.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un rastreo normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un sitio lleno de enlaces duplicados** y **Servidores lentos**. Tu diseño tiene que superar las tres.
