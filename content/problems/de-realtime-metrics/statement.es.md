# Métricas de producto en tiempo real

## El producto

Tu equipo de producto quiere un panel en vivo: altas, compras y vistas de pantalla, contadas cada 30 segundos, para que una mala versión se note en minutos y no en el informe de mañana. Tu app web y tu app móvil envían un evento cada vez que un usuario hace algo. Los eventos pasan por un Stream hasta un Procesador de streams, que los cuenta en ventanas de 30 segundos según el momento en que ocurrió cada evento (su tiempo de evento), no el momento en que llegó. Los recuentos van a una Base de datos de series temporales, y el panel los lee de ahí.

La app web envía sus eventos en uno o dos segundos. La app móvil es distinta: un teléfono en un túnel, en un avión o con la app en segundo plano guarda sus eventos y los envía cuando vuelve a tener conexión, a veces minutos después. Esos eventos siguen perteneciendo a la ventana en la que ocurrieron.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 5000 peticiones/s |
| Tiempo p95 hasta un evento contado | 45 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Eventos tardíos | 1 % o menos de los eventos contados en ventanas |
| Eventos perdidos | ninguno |
| Presupuesto | 2200 $ al mes |

## El tráfico

- **Un evento de la app web** (97 %). La petición puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones, que la publican en un Stream. El Procesador de streams la consume y la suma a su ventana. Los eventos web llegan unos 0,3 s después de ocurrir (3 s en el p99).
- **Un evento de la app móvil** (2 %). El mismo camino. Los eventos móviles llegan unos 5 s después de ocurrir en la mediana, pero la cola es larga: 120 s en el p99.
- **Abrir el panel de métricas en vivo** (1 %). La petición puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones, que leen los recuentos de la Base de datos de series temporales.

Hay unas 20 000 métricas (una por nombre de evento y pantalla).

## Cómo funcionan las ventanas

- **Ventana.** La Ventana del Procesador de streams es Fija (tumbling), con una Duración de la ventana de 30 s: cada evento cuenta en la ventana de 30 segundos en la que cae su tiempo de evento.
- **Cuándo sale un resultado.** Una ventana se cierra en su final más el Retraso de la marca de agua, y entonces salen sus recuentos. El tiempo hasta un evento contado se mide, para cada evento, desde que se envía hasta que el resultado de su ventana sale del procesador. Con una ventana de 30 s, eso llega a unos 35 s incluso para un evento que llega al momento, así que el objetivo de 45 s deja unos 10 s para todo lo demás.
- **Eventos tardíos.** Un evento que llega al procesador después de que su ventana se haya cerrado actualiza esa ventana si está dentro del Retraso permitido; la actualización sale al momento. Más allá del Retraso permitido, es un evento tardío: se cuenta como tardío y se deja fuera de los recuentos. No se pierde, pero tu panel está mal.
- **Rendimiento.** Cada hueco del procesador (Instancias × Concurrencia por instancia) procesa aquí unos 800 eventos por segundo, incluida su lectura del Stream. Un Stream entrega cada partición a un solo hueco a la vez, así que las Particiones del Stream limitan cuántos huecos pueden trabajar.

El simulador no envía las escrituras del procesador a la Base de datos de series temporales (unas pocas filas por métrica y ventana); conecta igualmente el Procesador de streams a ella, como lo haría tu pipeline real.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Los móviles recuperan la conexión: 20 veces los eventos móviles** y **Día de lanzamiento: 3 veces los eventos durante 40 s**. Tu diseño tiene que superar las tres.
